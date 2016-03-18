# ws-p2p
An example of a peer-to-peer network implementing causal ordering using WebSockets.

[![Build Status](https://travis-ci.org/jonalmeida/ws-p2p.svg?branch=master)](https://travis-ci.org/jonalmeida/ws-p2p)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)


## Build

1. Install [Rust](https://rust-lang.org) for your system.
2. Use Cargo to build the source:

```bash
cd ws-p2p/
cargo build
```

## Usage Example

Peer 1:

```bash
RUST_LOG=ws-p2p=info ./target/debug/ws-p2p \
                --server 127.0.0.1:3012    \ # Address to run this peer with.
                --demo 127.0.0.1:3014        # Address of the peer you want to be delayed (for demo-purposes).
```

Peer 2:

```bash
RUST_LOG=ws-p2p=info ./target/debug/ws-p2p \
                --server 127.0.0.1:3013    \
                ws://127.0.0.1:3012          # A list of peer's to connect with.
```

Peer 3:

```bash
RUST_LOG=ws-p2p=info ./target/debug/ws-p2p \
                --server 127.0.0.1:3014    \
                ws://127.0.0.1:3012        \ # A list of peer's to connect with.
                ws://127.0.0.1:3014
```

## Disclaimer

1. Dynamically adding peers hasn't been tested well, so expect it to fail instantly.
2. Leaving for peers hasn't been implemented. Theoretically, it should be straight-forward with this implementation. Currently, I'm not receiving a closed or errored websocket connection message as expected to begin adding this feature, so I've left it as is for now.
