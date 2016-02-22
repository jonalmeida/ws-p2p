extern crate ws;

//use std::cell::RefCell; //unused
use std::io;
use std::io::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::vec;

//use ws::Handler; //unused
use ws::CloseCode;
use ws::Error;
//use ws::ErrorKind; //unused
use ws::Message;
use ws::Handshake;
//use ws::Factory; //unused

fn main() {
    let client = Client::connect("ws://127.0.0.1:3012");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        client.send(line.unwrap());
    }
}

enum Event<T> {
    Connect(T),
    Disconnect
}

struct Client {
    out: ws::Sender,
    // Used for sending the ws::Sender back to main thread through the channel
    thread_out: mpsc::Sender<Event<ws::Sender>>,
    // A way to hold all connected clients to send all connected peers
    clients: vec::Vec<ws::Sender>,
}

impl Client {
    fn connect(url: &'static str) -> ws::Sender {
        let (tx, rx) = channel();
        let client = thread::spawn(move || {
            ws::connect(url, |out| {
                out.send("Client has connected..");
                println!("Connected to server..");
                Client { out: out, thread_out:tx.clone(), clients: Vec::new() }
            }).unwrap();
        });

        if let Event::Connect(sender) = rx.recv().unwrap() {
            sender
        } else {
            //TODO: Find a way to circumvent connection refused until the server is up.
            panic!("at the disco");
        }
    }
}

impl ws::Handler for Client {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        self.thread_out
			.send(Event::Connect(self.out.clone()))
			.map_err(|err| ws::Error::new(
                ws::ErrorKind::Internal,
                format!("Unable to communicate between threads: {:?}.", err)))
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        println!("Recv: {}", msg);
        Ok(io::stdout().flush().unwrap())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("Exited gracefully.."),
            _ => println!("We fucked up, Bob! {}", reason),
        }
    }

    fn on_error(&mut self, err: Error) {
        println!("We errored: {:?}", err);
    }
}

/*
//TODO: Implement a factory handler for starting the server
struct MyHandler {
	connected_client: ws::Sender,
}

impl ws::Handler for MyHandler {}

impl ws::Factory for Client {
    type Handler = Client;
    fn connection_made(&mut self, sender: ws::Sender) -> Self::Handler {
		self.clients.push(sender);
        *self
    }
}
*/
