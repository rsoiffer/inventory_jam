# Inventory Jam

A Bevy game with Lightyear multiplayer, based on the [bevy_game_template](https://github.com/NiklasEi/bevy_game_template).

## Multiplayer

This project uses [Lightyear](https://github.com/cBournhonesque/lightyear) with the same server-authoritative pattern as the [simple_box example](https://github.com/cBournhonesque/lightyear/tree/main/examples/simple_box):

- replicated player positions and colors
- client-side prediction for the local player
- interpolation for remote players
- WebTransport + Netcode for native and web clients

### Run a dedicated server

```bash
cargo run -- --server
```

The server listens on `127.0.0.1:5888` over WebTransport.

### Run a native client

```bash
cargo run
```

Use the menu:

- **Host** starts a local server and client in one app
- **Join** connects to `127.0.0.1:5888`

Optional CLI flags:

```bash
cargo run -- --server-addr 127.0.0.1:5888 -c 1
```

### Run a web client

Requires the `wasm32-unknown-unknown` target and [trunk](https://github.com/trunk-rs/trunk):

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve
```

Start a native dedicated server first, then open the web client and choose **Join**. The browser client connects to the current page hostname on port `5888`.

### WebTransport certificates

The repo includes a self-signed certificate in `certificates/` for local development. The WASM client embeds `certificates/digest.txt` at compile time, so regenerate and rebuild if you replace the certificate:

```bash
sh certificates/generate.sh
cargo build --target wasm32-unknown-unknown --no-default-features --features client
```

## Local development

- Native app (fast compiles): `cargo run-dev` or `cargo run --features dev`
- Native app: `cargo run`
- Web build: `trunk serve`
- Format: `cargo fmt`
- Check: `cargo check-dev` or `cargo check`

Dev builds use Bevy [dynamic linking](https://bevy.org/learn/quick-start/getting-started/setup/) and dependency optimizations from `Cargo.toml` to keep iterative compiles fast. On Windows, linking uses `rust-lld` via `.cargo/config.toml`.

If `rust-lld` is missing, run:

```bash
rustup component add llvm-tools-preview
```

## Template notes

See the original template docs below for icons, GitHub Pages deployment, and Bevy learning resources.

# A Bevy game template

Template for a Game using the awesome [Bevy engine][bevy] featuring out of the box builds for Windows, Linux, and Web (Wasm).

[bevy]: https://bevyengine.org/
[trunk]: https://github.com/trunk-rs/trunk
