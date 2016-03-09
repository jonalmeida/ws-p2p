extern crate ws;

use message::PeerMessage;
use std::collections::hash_map::HashMap;
use std::sync::{Arc, Mutex};

use bincode::rustc_serialize::decode;

/// Connection handler for incoming messages.
pub struct MessageHandler {
    /// Sender that is used to communicate for the handler.
    pub ws: ws::Sender,
    /// An arc clone of my local vector clocks.
    pub clocks: Arc<Mutex<HashMap<String, u32>>>,
    /// My address/name.
    pub me: String,
}

//TODO: When connected to another client, add it to the `clocks`.
impl ws::Handler for MessageHandler {
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

                info!("Peer {} with clocks: {:?} got message: {}",
                        message.sender, message.clocks, message.message);
                //message_checking(&mut clocks, message.clone());
                Ok(())
            },
            ws::Message::Text(string) => {
                let mut clocks = self.clocks.lock().unwrap();
                clocks.insert(string.clone(), 0u32);
                Ok(info!("Received peer's name. Adding {} to the client list", string))
            }
        }
    }
    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        if reason.is_empty() {
            info!("Client disconnected with code: {:?}", code); //This works: CloseCode::Abnormal
        } else {
            info!("Client disconnected with code: {:?} and reason: {}", code, reason);
        }
    }
    fn on_error(&mut self, err: ws::Error) {
        warn!("Error family robinson! {:?}", err.kind);
    }
}

pub struct MessageFactory {
    vclocks: Arc<Mutex<HashMap<String, u32>>>,
    me: String,
}

impl MessageFactory {
    pub fn build(vclocks: Arc<Mutex<HashMap<String, u32>>>) -> MessageFactory {
        MessageFactory {
            vclocks: vclocks.clone(),
            me: String::from("undefined"),
        }
    }
    pub fn me(&self, me: &str) -> MessageFactory {
        MessageFactory {
            vclocks: self.vclocks.clone(),
            me: String::from(me),
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
        }
    }

    fn server_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        ws.send(self.me.clone());
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
        }
    }

    fn client_connected(&mut self, ws: ws::Sender) -> Self::Handler {
        ws.send(self.me.clone());
        MessageHandler {
            ws: ws,
            clocks: self.vclocks.clone(),
            me: self.me.clone(),
        }
    }
}
