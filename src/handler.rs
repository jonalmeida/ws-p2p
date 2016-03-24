extern crate ws;

use message::PeerMessage;

use std::cmp;
use std::collections::hash_map::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bincode::rustc_serialize::decode;

use ws::util::Token;

static LIB_NAME: &'static str = "ws-p2p";

const DELAYED_MESSAGE: Token = Token(1);

/// Connection handler for incoming messages.
pub struct MessageHandler {
    /// Sender that is used to communicate for the handler.
    pub ws: ws::Sender,
    /// An arc clone of my local vector clocks.
    pub clocks: Arc<Mutex<HashMap<String, u32>>>,
    /// My address/name.
    pub me: String,
    /// Buffer of messages that caused conflicts when initially received.
    pub buffer: Arc<Mutex<VecDeque<PeerMessage>>>,
    /// Delays messages received from client.
    pub demo_client: Option<String>,
    /// Holder for delayed messages.
    message_queue: VecDeque<PeerMessage>,
}

impl ws::Handler for MessageHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        match msg {
            ws::Message::Binary(vector) => {
                let encoded_msg = &*vector.into_boxed_slice();
                if let Ok(message) = decode(encoded_msg) {
                    let message: PeerMessage = message;

                    debug!(target: LIB_NAME, "I am using {:?}.", self.ws.token());

                    if let Some(demo_client) = self.demo_client.clone() {
                        if message.sender.as_str() == demo_client.as_str() {
                            info!(target: LIB_NAME,
                                  "Faking delay for messages from {}!", demo_client);
                            self.message_queue.push_back(message);
                            return self.ws.timeout(4000, DELAYED_MESSAGE);
                        }
                    }

                    self.message_handler(message);
                    Ok(())
                } else {
                    Err(ws::Error::new(ws::ErrorKind::Internal,
                                       "Failed to serialize the binary message."))
                }
            }
            ws::Message::Text(string) => {
                info!(target: LIB_NAME, "Received peer's name. Adding {} to client list", string);
                if let Ok(mut clocks) = self.clocks.lock() {
                    clocks.insert(string, 0u32);
                    Ok(())
                } else {
                    Err(ws::Error::new(ws::ErrorKind::Internal,
                                       "Failed to serialize the string message."))
                }
            }
        }
    }
    fn on_timeout(&mut self, event: Token) -> ws::Result<()> {
        match event {
            DELAYED_MESSAGE => {
                if let Some(msg) = self.message_queue.pop_front() {
                    self.message_handler(msg);
                    Ok(())
                } else {
                    Err(ws::Error::new(ws::ErrorKind::Internal,
                                       "Invalid timeout token encountered!"))
                }
            }
            _ => Err(ws::Error::new(ws::ErrorKind::Internal,
                                    "Invalid timeout token encountered!")),
        }
    }
    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        // TODO: Remove disconnected client's clock from our vclock copy.
        // We don't get to here because our stdin thread doesn't pass
        // the SIGINT to the main thread when closing. This should work
        // as a library without a CLI.
        info!(target: LIB_NAME, "Received an on_close call");
        if reason.is_empty() {
            // This works: CloseCode::Abnormal
            info!(target: LIB_NAME, "Client disconnected with code: {:?}", code);
        } else {
            info!(target: LIB_NAME, "{} d/c with code: {:?} and reason: {}", self.me, code, reason);
        }
    }
    fn on_error(&mut self, err: ws::Error) {
        warn!(target: LIB_NAME, "Error family robinson! {:?}", err.kind);
    }
    fn on_shutdown(&mut self) {
        warn!(target: LIB_NAME, "Socket shutdown from {}", self.me);
    }
}

impl MessageHandler {
    fn buffer_check(&self) {
        let mut buffer = self.buffer.lock().unwrap();
        let buffer_clone = buffer.clone();
        for message in buffer_clone.iter() {
            let mut vclocks = self.clocks.lock().unwrap();
            for (key, val) in message.clocks.iter() {
                if key.as_str() == message.sender.as_str() {
                    if let Some(lval) = vclocks.get(&key.clone()) {
                        if *val != *lval + 1 {
                            debug!(target: LIB_NAME, "discrepency still exists");
                            return;
                        }
                    }
                } else {
                    if let Some(lval) = vclocks.get(&key.clone()) {
                        debug!(target: LIB_NAME, "key: {} val: {} and lval: {}", key, val, lval);
                        if val <= lval {
                            debug!(target: LIB_NAME, "val <= lval");
                            continue;
                        } else {
                            // There's a clock that is greater than what we have
                            debug!(target: LIB_NAME, "discrepency still exists");
                            return;
                        }
                    }
                }
            }

            // Update clocks
            for (key, val) in message.clocks.iter() {
                if vclocks.get(&key.clone()).is_some() {
                    let lval = vclocks.get(&key.clone()).unwrap().clone();
                    let max_val = cmp::max(val, &lval);
                    vclocks.insert(key.clone(), *max_val);
                }
            }
            info!(target: LIB_NAME, "buf_check: Peer {} with clocks: {:?} got message: {}",
                    message.sender, message.clocks, message.message);
            buffer.pop_front();
        }
    }
    fn message_handler(&self, message: PeerMessage) {
        // Finishes the lifetime of the mutex holds so buffer_check can use it without
        // blocking
        {
            let mut vclocks = self.clocks.lock().unwrap();
            for (key, val) in message.clocks.iter() {
                if key.as_str() == message.sender.as_str() {
                    if let Some(lval) = vclocks.get(&key.clone()) {
                        if *val != *lval + 1 {
                            if let Ok(mut buffer) = self.buffer.lock() {
                                debug!(target: LIB_NAME, "pushing to buffer..");
                                buffer.push_back(message.clone());
                            }
                            debug!(target: LIB_NAME, "clock discrepency with from incoming peer");
                            return;
                        }
                    }
                } else {
                    if let Some(lval) = vclocks.get(&key.clone()) {
                        debug!(target: LIB_NAME, "key: {} val: {} and lval: {}", key, val, lval);
                        if val <= lval {
                            debug!(target: LIB_NAME, "val <= lval");
                            continue;
                        } else {
                            // There's a clock that is greater than what we have
                            if let Ok(mut buffer) = self.buffer.lock() {
                                debug!(target: LIB_NAME, "pushing to buffer..");
                                buffer.push_back(message.clone());
                            }
                            debug!(target: LIB_NAME, "clock discrepency from other peer clocks");
                            return;
                        }
                    }
                }
            }

            // Update clocks
            debug!(target: LIB_NAME, "Updating clocks now because no discrepency");
            info!(target: LIB_NAME, "msg_handler: Peer {} with clocks: {:?} got message: {}",
                    message.sender, message.clocks, message.message);
            for (key, val) in message.clocks.iter() {
                vclocks.insert(key.clone(), *val);
            }
        }
        self.buffer_check();
    }
}

#[derive(Clone)]
pub struct MessageFactory {
    vclocks: Arc<Mutex<HashMap<String, u32>>>,
    me: String,
    buffer: Arc<Mutex<VecDeque<PeerMessage>>>,
    demo_client: Option<String>,
}

impl MessageFactory {
    pub fn build(vclocks: Arc<Mutex<HashMap<String, u32>>>) -> MessageFactory {
        MessageFactory {
            vclocks: vclocks.clone(),
            me: String::from("undefined"),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            demo_client: None,
        }
    }
    pub fn me(&self, me: &str) -> MessageFactory {
        MessageFactory {
            vclocks: self.vclocks.clone(),
            me: String::from(me),
            buffer: self.buffer.clone(),
            demo_client: match self.demo_client.clone() {
                Some(peer) => Some(peer),
                None => None,
            },
        }
    }
    pub fn demo(&mut self, peer: &str) {
        self.demo_client = Some(String::from(peer));
    }
}

impl ws::Factory for MessageFactory {
    type Handler = MessageHandler;

    fn connection_made(&mut self, ws: ws::Sender) -> Self::Handler {
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
            buffer: self.buffer.clone(),
            demo_client: self.demo_client.clone(),
            message_queue: VecDeque::new(),
        }
    }

    fn server_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        if let Err(e) = ws.send(self.me.clone()) {
            panic!("Unable to send name to connected peer. Something is wrong..\n{}",
                   e);
        }
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
            buffer: self.buffer.clone(),
            demo_client: self.demo_client.clone(),
            message_queue: VecDeque::new(),
        }
    }

    fn client_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        if let Err(e) = ws.send(self.me.clone()) {
            panic!("Unable to send name to connected peer. Something is wrong..\n{}",
                   e);
        }
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
            buffer: self.buffer.clone(),
            demo_client: self.demo_client.clone(),
            message_queue: VecDeque::new(),
        }
    }
}
