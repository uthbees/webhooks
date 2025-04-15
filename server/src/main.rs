use axum::routing::any;
use axum::Router;
use websockets_server::ws_handler;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", any(ws_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
