use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use actix_web::{web, Error, HttpRequest, HttpResponse};
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

    fn add_session(
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

async fn handle_ws(
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

pub async fn echo_ws(
    req: HttpRequest,
    body: web::Payload,
    ws_connections: web::Data<Arc<RwLock<WsConnections>>>,
) -> actix_web::Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        println!("connected");
        let session = ws_connections
            .into_inner()
            .write()
            .unwrap()
            .add_session("client", session);
        handle_ws(session, msg_stream).await;
    });

    return Ok(res);
}
