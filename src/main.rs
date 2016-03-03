/// An example of a client/server agnostic WebSocket that takes input from stdin and sends that
/// input to all other peers.
///
/// For example, to create a network like this:
///
/// 3013 ---- 3012 ---- 3014
///   \        |
///    \       |
///     \      |
///      \     |
///       \    |
///        \   |
///         \  |
///          3015
///
/// Run these commands in separate processes
/// ./peer2peer
/// ./peer2peer --server localhost:3013 ws://localhost:3012
/// ./peer2peer --server localhost:3014 ws://localhost:3012
/// ./peer2peer --server localhost:3015 ws://localhost:3012 ws://localhost:3013
///
/// Stdin on 3012 will be sent to all other peers
/// Stdin on 3013 will be sent to 3012 and 3015
/// Stdin on 3014 will be sent to 3012 only
/// Stdin on 3015 will be sent to 3012 and 2013

extern crate ws;
extern crate url;
extern crate clap;
extern crate env_logger;
extern crate rustc_serialize;
extern crate bincode;
#[macro_use] extern crate log;

use std::clone::Clone;
use std::io;
use std::io::prelude::*;
use std::thread;

use bincode::rustc_serialize::{encode, decode};

use clap::{App, Arg};

fn main() {
    // Setup logging
    env_logger::init().unwrap();

    // Parse command line arguments
    let matches = App::new("Simple Peer 2 Peer")
        .version("1.0")
        .author("Jonathan Almeida <hello@jonalmeida.com")
        .about("Connect to other peers and listen for incoming connections.")
        .arg(Arg::with_name("server")
             .short("s")
             .long("server")
             .value_name("SERVER")
             .help("Set the address to listen for new connections. (default: localhost:3012"))
        .arg(Arg::with_name("PEER")
             .help("A WebSocket URL to attempt to connect to at start.")
             .multiple(true))
        .get_matches();

    // Get address of this peer
    let my_addr = matches.value_of("server").unwrap_or("localhost:3012");
    let sending_addr = String::new() + my_addr;

    // Create simple websocket that just prints out messages
    let mut me = ws::WebSocket::new(|sender| {
        move |msg| {
            match msg {
                ws::Message::Binary(vector) => {
                    let encoded_msg = &*vector.into_boxed_slice();
                    let message: PeerMessage = decode(encoded_msg).unwrap();
                    Ok(info!("Peer {} got message: {}", message.sender, message.message))
                },
                ws::Message::Text(string) => {
                    Ok(warn!("We received a string, but we don't want to handle it: {}", string))
                },
            }
        }
    }).unwrap();

    // Get a sender for ALL connections to the websocket
    let broacaster = me.broadcaster();

    // Setup thread for listening to stdin and sending messages to connections
    let input = thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            // Send a message to all connections regardless of
            // how those connections were established
            let clocks = vec![0u32, 1u32, 1u32];
            let message = PeerMessage { sender: sending_addr.clone(), clocks: clocks, message: line.unwrap() };
            let encoded: Vec<u8> = encode(&message, bincode::SizeLimit::Infinite).unwrap();
            //broacaster.send(line.unwrap()).unwrap();
            broacaster.send(&*encoded.into_boxed_slice()).unwrap();
        }
    });

    // Connect to any existing peers specified on the cli
    if let Some(peers) = matches.values_of("PEER") {
        for peer in peers {
            me.connect(url::Url::parse(peer).unwrap()).unwrap();
        }
    }

    // Run the websocket
    me.listen(my_addr).unwrap();
    input.join().unwrap();

}

#[derive(RustcEncodable, RustcDecodable, PartialEq)]
struct PeerMessage {
    // Host address passed in via CLI. Used for identification.
    sender: String,
    // Vector clocks for each process.
    clocks: Vec<u32>,
    // The message that we want to send. For now, we're testing with a string.
    message: String,
}
