extern crate rustc_serialize;
extern crate bincode;

use std::collections::hash_map::HashMap;

#[derive(RustcEncodable, RustcDecodable, PartialEq, Clone)]
pub struct PeerMessage {
    // Host address passed in via CLI. Used for identification.
    pub sender: String,
    // Vector clocks for each process.
    pub clocks: HashMap<String, u32>,
    // The message that we want to send. For now, we're testing with a string.
    pub message: String,
}
