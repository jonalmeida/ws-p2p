#### Messages sent and received over a WebSocket.

```rust
pub struct PeerMessage {
/// Host address passed in via CLI. Used for identification.
    pub sender: String,
/// Vector clocks for each process.
    pub clocks: HashMap<String, u32>,
/// The message that we want to send.
    pub message: String,
}

```
## Note
- For implementation simplicity, encodes with an "unlimited" message size.