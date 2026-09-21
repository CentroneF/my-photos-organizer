# Tauri + Dioxus

This template should help get you started developing with Tauri and Dioxus.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) + [Dioxus](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus).

## macOS setup and development

Install Xcode Command Line Tools if they are not already available:

```sh
xcode-select --install
```

Install Rust and Cargo:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Choose the default installation, then open a new terminal or load Cargo into the
current shell:

```sh
source "$HOME/.cargo/env"
```

Install the Tauri 2 CLI required by this project:

```sh
cargo install tauri-cli --version '^2' --locked
```

Install the Dioxus 0.6 CLI used to serve the frontend during development:

```sh
cargo install dioxus-cli --version '^0.6' --locked
```

From the repository root, start the desktop application:

```sh
cargo tauri dev
```

This command automatically starts the Dioxus development server with
`dx serve --port 1420` before launching Tauri. Do not run that command in a
separate terminal at the same time: it would conflict on port 1420.
