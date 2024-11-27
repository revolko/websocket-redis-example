use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_ws::Message;

pub struct WsConnections {
    pub sessions: HashMap<String, Arc<Mutex<actix_ws::Session>>>,
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

    pub fn add_session(
        &mut self,
        session_id: &str,
        session: actix_ws::Session,
    ) -> Arc<Mutex<actix_ws::Session>> {
        let safe_session = Arc::new(Mutex::new(session));
        self.sessions
            .insert(session_id.to_string(), safe_session.clone());

        return safe_session;
    }
}

pub async fn handle_ws(
    session: Arc<Mutex<actix_ws::Session>>,
    mut msg_stream: actix_ws::MessageStream,
) {
    while let Some(msg) = msg_stream.recv().await {
        println!("got message");
        match msg {
            Ok(Message::Text(text)) => {
                println!("got text");
                session.lock().unwrap().text(text).await.unwrap();
            }
            Ok(Message::Ping(msg)) => {
                session.lock().unwrap().pong(&msg).await.unwrap();
            }
            _ => {}
        }
    }
}
