use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::CloseFrame;
use axum::{
    body::Bytes,
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use axum_extra::{TypedHeader, headers};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use std::ops::ControlFlow;

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");

    // Finalize the upgrade process by returning an upgrade callback.
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

/// Actual websocket state machine (one will be spawned per connection).
async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    // Send a ping (unsupported by some browsers) just to kick things off and get a response.
    if socket
        .send(Message::Ping(Bytes::from_static(&[1, 2, 3])))
        .await
        .is_ok()
    {
        println!("Pinged {who}...");
    } else {
        println!("Could not ping {who}!");
        // Just close the connection - if we can't send messages, there's nothing we can do.
        return;
    }

    // Receive a single message from a client (we can either receive or send with socket).
    // This will likely be the Pong for our Ping or a hello message from the client.
    // Waiting for a message from a client will block this task, but will not block other client's
    // connections.
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who).is_break() {
                return;
            }
        } else {
            println!("Client {who} disconnected abruptly.");
            return;
        }
    }

    // Since each client gets an individual state machine, we can pause handling when necessary
    // to wait for some external event (in this case illustrated by sleeping).
    // Waiting for this client to finish getting its greetings does not prevent other clients from
    // connecting to server and receiving their greetings.
    for i in 1..=5 {
        if socket
            .send(Message::Text(format!("Hi {i} times!").into()))
            .await
            .is_err()
        {
            println!("Client {who} disconnected abruptly.");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // By splitting the socket we can send and receive at the same time. In this example we will
    // send unsolicited messages to the client based on some sort of server-side event (ie a timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (it doesn't matter what the client does).
    let mut send_task = tokio::spawn(async move {
        let n_messages = 20;
        for i in 0..n_messages {
            // In case of any websocket error, we exit.
            if sender
                .send(Message::Text(format!("Server message {i}...").into()))
                .await
                .is_err()
            {
                return i;
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        println!("Sending close message to {who}...");
        if let Err(e) = sender
            .send(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Utf8Bytes::from_static("Goodbye"),
            })))
            .await
        {
            println!("Could not send Close due to {e}, but it's probably fine?");
        }
        n_messages
    });

    // This second task will receive messages from the client and print them on the server console.
    let mut recv_task = tokio::spawn(async move {
        let mut count = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            count += 1;
            // print message and break if instructed to do so
            if process_message(msg, who).is_break() {
                break;
            }
        }
        count
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        send_task_result = &mut send_task => {
            match send_task_result {
                Ok(result) => println!("{result} messages sent to {who}"),
                Err(result) => println!("Error sending messages: {result:?}")
            }
            recv_task.abort();
        },
        recv_task_result = &mut recv_task => {
            match recv_task_result {
                Ok(result) => println!("Received {result} messages"),
                Err(result) => println!("Error receiving messages: {result:?}")
            }
            send_task.abort();
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}

/// Utility function to print the contents of a message to stdout. Returns Break if the client
/// closed the socket.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent string: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> {who} sent {} bytes: {d:?}", d.len());
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {who} sent close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }
        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        // We don't need to manually handle Message::Ping since axum will automatically respond
        // with a Pong for us.
        Message::Ping(_) => {}
    }
    ControlFlow::Continue(())
}
