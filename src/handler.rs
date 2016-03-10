extern crate ws;

use message::PeerMessage;

use std::cmp;
use std::collections::hash_map::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use bincode::rustc_serialize::decode;

static LIB_NAME: &'static str = "ws-p2p";

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
}

impl ws::Handler for MessageHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        match msg {
            ws::Message::Binary(vector) => {
                let encoded_msg = &*vector.into_boxed_slice();
                let message: PeerMessage = decode(encoded_msg).unwrap();

                info!(target: LIB_NAME, "Peer {} with clocks: {:?} got message: {}",
                        message.sender, message.clocks, message.message);
                if message.sender.as_str() == "127.0.0.1:3013" {
                    info!(target: LIB_NAME, "Faking delay!");
                    thread::sleep(Duration::from_millis(4000))
                }
                //message_checking(&mut clocks, message.clone());
				self.message_handler(message);
                self.buffer_check();
                Ok(())
            },
            ws::Message::Text(string) => {
                info!(target: LIB_NAME, "Received peer's name. Adding {} to the client list", string);
                let mut clocks = self.clocks.lock().unwrap();
                clocks.insert(string, 0u32);
                Ok(())
            }
        }
    }
    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        //TODO: Remove disconnected client's clock from our vclock copy.
        if reason.is_empty() {
            //This works: CloseCode::Abnormal
            info!(target: LIB_NAME, "Client disconnected with code: {:?}", code);
        } else {
            info!(target: LIB_NAME, "Client disconnected with code: {:?} and reason: {}", code, reason);
        }
    }
    fn on_error(&mut self, err: ws::Error) {
        warn!(target: LIB_NAME, "Error family robinson! {:?}", err.kind);
    }
}

impl MessageHandler {
    fn buffer_check(&self) {
        debug!("trying to get lock");
        let mut buffer = self.buffer.lock().unwrap();
        let buffer_clone = buffer.clone();
        for message in buffer_clone.iter() {
            debug!("Are we getting here?");
            let mut vclocks = self.clocks.lock().unwrap();
            for (key, val) in message.clocks.iter() {
                if let Some(lval) = vclocks.get(&key.clone()) {
                    debug!(target: LIB_NAME, "key: {} incoming val: {} and lval: {}", key, val, lval);
                    if lval <= val {
                        debug!(target: LIB_NAME, "val <= lval");
                        continue;
                    } else { // There's a clock that is greater than what we have
                        debug!(target: LIB_NAME, "discrepency still exists");
                        ()
                    }
                }
            }

            // Update clocks
            for (key, val) in message.clocks.iter() {
                let lval = vclocks.get(&key.clone()).unwrap().clone();
                let max_val = cmp::max(val, &lval);
                vclocks.insert(key.clone(), *max_val);
                info!(target: LIB_NAME, "Peer {} with clocks: {:?} got message: {}",
                        message.sender, message.clocks, message.message);
            }
            buffer.pop_front();
        }
    }
    fn message_handler(&self, message: PeerMessage) {
        let mut vclocks = self.clocks.lock().unwrap();
        for (key, val) in message.clocks.iter() {
            if let Some(lval) = vclocks.get(&key.clone()) {
                debug!(target: LIB_NAME, "key: {} incoming val: {} and lval: {}", key, val, lval);
                if lval <= val {
                    debug!(target: LIB_NAME, "val <= lval");
                    continue;
                } else { // There's a clock that is greater than what we have
                    // Push to local buffer
                    let mut buffer = self.buffer.lock().unwrap();
                    // return
                    debug!(target: LIB_NAME, "clock discrepency");
                    //self.buffer_check();
                    ()
                }
            }
        }

        // Update clocks
        for (key, val) in message.clocks.iter() {
            let lval = vclocks.get(&key.clone()).unwrap().clone();
            let max_val = cmp::max(val, &lval);
            vclocks.insert(key.clone(), *max_val);
        }
    //	macro_rules! hashmap {
    //		($( $key: expr => $val: expr ),*) => {{
    //			 let mut map = ::std::collections::HashMap::new();
    //			 $( map.insert($key, $val); )*
    //			 map
    //		}}
    //	}
        //let map = hashmap!['a' => 0, 'b' => 1];
    }
}

pub struct MessageFactory {
    vclocks: Arc<Mutex<HashMap<String, u32>>>,
    me: String,
    buffer: Arc<Mutex<VecDeque<PeerMessage>>>,
}

impl MessageFactory {
    pub fn build(vclocks: Arc<Mutex<HashMap<String, u32>>>) -> MessageFactory {
        MessageFactory {
            vclocks: vclocks,
            me: String::from("undefined"),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub fn me(&self, me: &str) -> MessageFactory {
        MessageFactory {
            vclocks: self.vclocks.clone(),
            me: String::from(me),
            buffer: self.buffer.clone(),
        }
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
        }
    }

    fn server_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        ws.send(self.me.clone()).unwrap();
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
            buffer: self.buffer.clone(),
        }
    }

    fn client_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        ws.send(self.me.clone()).unwrap();
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
            buffer: self.buffer.clone(),
        }
    }
}

