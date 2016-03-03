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

use std::collections::hash_map::HashMap;
use std::clone::Clone;
use std::io;
use std::io::prelude::*;
use std::thread;
use std::sync::{Arc, Mutex, MutexGuard};

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
    let sending_addr = String::from(my_addr);

    let mut clock = HashMap::new();
    clock.insert(sending_addr.clone(), 0u32);
    let clocks = Arc::new(Mutex::new(clock));
    let connecting_clocks = clocks.clone();
    let address_copy = sending_addr.clone();

    // Create simple websocket that just prints out messages
    let mut me = ws::WebSocket::new(|sender| {
        MyHandler { ws: sender, clocks: connecting_clocks.clone(), me: address_copy.clone() }
    }).unwrap();

    // Get a sender for ALL connections to the websocket
    let broacaster = me.broadcaster();

    // Setup thread for listening to stdin and sending messages to connections
    let input = thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            // Send a message to all connections regardless of
            // how those connections were established

            // TO-DO: Increment my clock here before sending it.
            let clock_clone = clocks.clone();
            let mut clock_lock = clock_clone.lock().unwrap();
            *clock_lock.get_mut(&sending_addr.clone()).unwrap() += 1;

            let message =
                PeerMessage {
                    sender: sending_addr.clone(),
                    clocks: clock_lock.clone(),
                    message: line.unwrap()
                };
            let encoded: Vec<u8> = encode(&message, bincode::SizeLimit::Infinite).unwrap();

            // Sending bytes to all connected clients
            broacaster.send(&*encoded.into_boxed_slice()).unwrap();
        }
    });

    // Connect to any existing peers specified on the cli
    if let Some(peers) = matches.values_of("PEER") {
        for peer in peers {
            let addrs = peer.clone();
            me.connect(url::Url::parse(peer).unwrap()).unwrap();
        }
    }

    // Run the websocket
    me.listen(my_addr).unwrap();
    input.join().unwrap();

}

struct MyHandler {
    // Sender that is used to communicate for my handler
    ws: ws::Sender,
    // An arc clone of my local vector clocks
    clocks: Arc<Mutex<HashMap<String, u32>>>,
    // My address/name
    me: String,
}

impl ws::Handler for MyHandler {
    fn on_open(&mut self, handshake: ws::Handshake) -> ws::Result<()> {
        let mut clocks = self.clocks.lock().unwrap();
        let addr = handshake.request.client_addr().unwrap();
        warn!("Connected with {:?}.", addr);
        if let Some(addrs) = addr {
            //warn!("Connected with {}.", addrs);
            clocks.insert(String::from(addrs), 0u32);
        }
        Ok(())
    }
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        match msg {
            ws::Message::Binary(vector) => {
                let encoded_msg = &*vector.into_boxed_slice();
                let message: PeerMessage = decode(encoded_msg).unwrap();

                // Checkings if all clients have everyone's clocks
                let mut clocks = self.clocks.lock().unwrap();
                for (key, val) in message.clocks.iter() {
                    let value = val.clone();
                    let keyer = key.clone();
                    if !clocks.contains_key(&keyer) {
                        clocks.insert(String::from(keyer), value);
                    }
                }

                message_checking(&mut clocks, message.clone());
                Ok(())
            },
            ws::Message::Text(string) => Ok(warn!("Received a string, but don't want to handle it: {}", string)),
        }
    }
    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        if reason.is_empty() {
            info!("Client disconnected with code: {:?}", code);
        } else {
            info!("Client disconnected with code: {:?} and reason: {}", code, reason);
        }
    }
    fn on_error(&mut self, err: ws::Error) {
        info!("Error family robinson!");
    }
}

#[derive(RustcEncodable, RustcDecodable, PartialEq, Clone)]
struct PeerMessage {
    // Host address passed in via CLI. Used for identification.
    sender: String,
    // Vector clocks for each process.
    clocks: HashMap<String, u32>,
    // The message that we want to send. For now, we're testing with a string.
    message: String,
}

fn message_checking(local_clocks: &mut MutexGuard<HashMap<String, u32>>, incoming_message: PeerMessage) {
    let local_clock = local_clocks;
    let message_clocks = incoming_message.clocks.clone();
    for (key, val) in message_clocks.iter() {
        info!("Looking at key: {}", key);
        //let key_clone = key.clone();
        let sender_clone = incoming_message.sender.clone();
        if !message_clocks.contains_key(&key.clone()) {
            local_clock.insert(String::from(key.clone()), 0u32);
        }
        if key.clone().into_boxed_str() == sender_clone.clone().into_boxed_str() {
            //local_clocks.get(&key_clone).unwrap()
            info!("Looking at val: {}", *val);
            if *val != *local_clock.get(&sender_clone.clone()).unwrap() + 1 {
                info!("Buffering.. message: {}", incoming_message.message);
                // buffer.push_back
                ()
            }
        } else {
            if *val == *local_clock.get(&sender_clone.clone()).unwrap() {
                //buffer.flush
                continue;
            } else {
                //buffer.push();
                ()
            }
            //approve_new_messages();
        }
    }
    for (key, val) in message_clocks.iter() {
        local_clock.insert(key.clone(), val.clone());
    }
    info!("Peer {} with clocks: {:?} got message: {}",
            incoming_message.sender, incoming_message.clocks, incoming_message.message);
}

