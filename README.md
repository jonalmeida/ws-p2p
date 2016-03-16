# ws-p2p
An example of a peer-to-peer network imlementing causal ordering using WebSockets


# Build

1. Install [Rust](https://rust-lang.org) for your system.
2. Use Cargo to build the source:

```bash
cd ws-p2p/
cargo build
```

# Usage Example

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

