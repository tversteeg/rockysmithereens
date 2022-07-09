<h1 align="center">rockysmithereens</h1>
<p align="center">
   Rocksmith CDLC (`.psarc`) player.
</p>

<p align="center">
   <a href="https://actions-badge.atrox.dev/tversteeg/rockysmithereens/goto"><img src="https://github.com/tversteeg/rockysmithereens/workflows/CI/badge.svg" alt="Build Status"/></a>
   <a href="https://github.com/tversteeg/rockysmithereens/releases"><img src="https://img.shields.io/crates/d/rockysmithereens.svg" alt="Downloads"/></a>
   <a href="https://crates.io/crates/rockysmithereens"><img src="https://img.shields.io/crates/v/rockysmithereens.svg" alt="Version"/></a>
   <br/><br/>
</p>

## Player

```bash
cargo run
```

## Tools

### Play song from Rocksmith `.psarc`

```bash
cargo run --bin cli_music_player -- example_file.psarc
```

### Extract from `.psarc`

```bash
# Show files in archive
cargo run --bin psarc_extract -- example_file.psarc list

# Extract a file from the archive
cargo run --bin psarc_extract -- example_file.psarc extract example/path/from/above/command output_file.ext 
```

## Build

### WebAssembly

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-name rockysmithereens --out-dir web --target web target/wasm32-unknown-unknown/release/rockysmithereens.wasm

cargo install basic-http-server
(cd web && basic-http-server)
```
