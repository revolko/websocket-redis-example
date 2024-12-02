use core::panic;
use std::collections::HashMap;
use std::pin::pin;

use actix_ws::Message;
use futures_util::{
    future::{select, Either},
    StreamExt,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct WsConnections {
    pub sessions: HashMap<String, UnboundedSender<String>>,
}

impl Default for WsConnections {
    fn default() -> Self {
        return WsConnections {
            sessions: HashMap::new(),
        };
    }
}

impl WsConnections {
    pub fn new() -> Self {
        return WsConnections::default();
    }

    pub fn add_session(&mut self, session_id: &str, tx: UnboundedSender<String>) -> () {
        self.sessions.insert(session_id.to_string(), tx);
    }
}

pub async fn handle_ws(
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
    mut redis_rx: UnboundedReceiver<String>,
) {
    let mut msg_stream = pin!(msg_stream);
    loop {
        let redis_msg = pin!(redis_rx.recv());
        let ws_msgs = select(msg_stream.next(), redis_msg);

        match ws_msgs.await {
            Either::Left((Some(ws_msg), _)) => match ws_msg {
                Ok(Message::Text(text)) => {
                    session.text(text).await.unwrap();
                }
                Ok(Message::Ping(msg)) => {
                    session.pong(&msg).await.unwrap();
                }
                Ok(Message::Close(_)) => {
                    session.close(None).await.unwrap();
                    // TODO close redis_rx; it should be possible to open the sink once again,
                    // although not preferable as it would get broken with multiple replicas
                    break;
                }
                _ => {}
            },
            // TODO: deal with None messages from websocket client
            Either::Left((None, _)) => panic!("got none in weboscket"),
            Either::Right((Some(redis_msg), _)) => {
                session.text(redis_msg).await.unwrap();
            }
            // TODO: deal with None messages from redis worker
            Either::Right((None, _)) => panic!("got none in redis messages"),
        }
    }
}
