# Miden Private Transport Client CLI

This binary allows the user to interact with the Miden Transport Layer via a simple command-line interface (CLI).
It is a wrapper around the Miden Transport Layer Client (Rust), allowing to send and fetch notes.

## Build

To build from source, run
```sh
cargo build --release --locked
```

The binary will be available on `./target/release/miden-private-transport-cli`.

## Usage

Send a note with `miden-private-transport-cli send --note [your_note_hex] --recipient [recipient_address_bech32]`.

Fetch notes for a given tag with `miden-private-transport-cli fetch --tag [tag_integer]`.

A local `*.sqlite` database file will be created for client persistence.

## License
This project is [MIT licensed](../../LICENSE).
