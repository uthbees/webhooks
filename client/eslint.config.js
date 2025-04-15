import js from '@eslint/js';
import globals from 'globals';
import reactPlugin from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import unusedImports from 'eslint-plugin-unused-imports';
import tseslint from 'typescript-eslint';

export default tseslint.config({
    ignores: ['build'],
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
        ecmaVersion: 2020,
        sourceType: 'module',
        globals: globals.browser,
        parserOptions: {
            projectService: true,
            tsconfigRootDir: import.meta.dirname,
        },
    },
    plugins: {
        react: reactPlugin,
        'react-hooks': reactHooks,
        'react-refresh': reactRefresh,
        'unused-imports': unusedImports,
    },
    extends: [
        js.configs.recommended,
        tseslint.configs.strictTypeChecked,
        tseslint.configs.stylisticTypeChecked,
    ],
    rules: {
        ...reactPlugin.configs.recommended.rules,
        ...reactHooks.configs.recommended.rules,
        'prefer-const': 'warn',
        '@typescript-eslint/no-shadow': 'warn',
        'no-param-reassign': 'warn',
        'unused-imports/no-unused-imports': 'warn',
        '@typescript-eslint/no-unused-vars': [
            'warn',
            {
                ignoreRestSiblings: true,
            },
        ],
        'no-nested-ternary': 'error',
        curly: ['error', 'multi-line'],
        'react-refresh/only-export-components': [
            'error',
            { allowConstantExport: true },
        ],
        '@typescript-eslint/no-confusing-void-expression': 'off',
        'react/react-in-jsx-scope': 'off',
    },
});
