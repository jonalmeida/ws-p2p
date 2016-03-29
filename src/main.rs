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
#[macro_use]
extern crate log;

pub mod handler;
pub mod message;

use message::PeerMessage;
use handler::MessageFactory;

use std::collections::hash_map::HashMap;
use std::clone::Clone;
use std::io;
use std::io::prelude::*;
use std::thread;
use std::sync::{Arc, Mutex};

use bincode::rustc_serialize::encode;

use clap::{App, Arg};

fn main() {
    // Setup logging
    env_logger::init().unwrap();

    // Parse command line arguments
    let matches = App::new("Simple Peer 2 Peer")
                      .version("1.0")
                      .author("Jonathan Almeida <hello@jonalmeida.com>")
                      .about("Connect to other peers and listen for incoming connections.")
                      .arg(Arg::with_name("server")
                               .short("s")
                               .long("server")
                               .value_name("SERVER")
                               .help("Set the address to listen for new connections. (default: \
                                      localhost:3012"))
                      .arg(Arg::with_name("PEER")
                               .help("A WebSocket URL to attempt to connect to at start.")
                               .multiple(true))
                      .arg(Arg::with_name("demo")
                               .short("d")
                               .long("demo")
                               .value_name("DEMO_SERVER")
                               .help("Sleeps for 4 seconds before receiving messages from this \
                                      peer."))
                      .arg(Arg::with_name("drop")
                               .short("r")
                               .long("drop")
                               .multiple(true)
                               .help("Drops messages instead of delays."))
                      .get_matches();

    // Get address of this peer
    let my_addr = String::from(matches.value_of("server").unwrap_or("localhost:3012"));
    let demo_addr = matches.value_of("demo");
    let my_addr_clone = my_addr.clone();

    let mut clock = HashMap::new();
    clock.insert(my_addr.clone(), 0u32);
    let clock = Arc::new(Mutex::new(clock));

    // Creating a Factory for the WebSocket
    let mut factory = MessageFactory::build(clock.clone()).me(my_addr.clone().as_str());
    if let Some(demo) = demo_addr {
        factory.demo(demo);
    }
    match matches.occurrences_of("r") {
        0 => (),
        _ => factory.drop(true),
    }

    // Create simple websocket that just prints out messages
    let mut me = ws::WebSocket::new(factory).unwrap();

    // Get a sender for ALL connections to the websocket
    let broacaster = me.broadcaster();

    // Setup thread for listening to stdin and sending messages to connections
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {

            // Incrementing clock before sending
            if let Ok(mut clock) = clock.lock() {
                if let Some(clock) = clock.get_mut(&my_addr_clone) {
                    *clock += 1;
                }

                // Constructing new message
                let message = PeerMessage {
                    sender: my_addr_clone.clone(),
                    clocks: clock.clone(),
                    message: line.unwrap_or(String::from("n/a")),
                };

                // Encoding into bytes
                if let Ok(encoded_msg) = encode(&message, bincode::SizeLimit::Infinite) {
                    // Sending bytes to all connected clients
                    broacaster.send(&*encoded_msg.into_boxed_slice()).unwrap();
                }
            }
        }
    });

    // Connect to any existing peers specified on the cli
    if let Some(peers) = matches.values_of("PEER") {
        for peer in peers {
            if let Ok(peer) = url::Url::parse(peer) {
                me.connect(peer).unwrap();
            }
        }
    }

    // Run the websocket
    if let Err(e) = me.listen(my_addr.clone().as_str()) {
        panic!("Failed to start WebSocket.\n{}", e);
    }
}
