use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, RwLock};

use actix_web::web::{self, Data};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::Message;

mod redis_worker;
use crate::redis_worker::spawn_redis_worker;

const REPLICA_ID: i8 = 1;

struct WsConnections {
    sessions: HashMap<String, Arc<Mutex<actix_ws::Session>>>,
}

impl Default for WsConnections {
    fn default() -> Self {
        return WsConnections {
            sessions: HashMap::new(),
        };
    }
}

impl WsConnections {
    fn new() -> Self {
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

async fn handle(session: Arc<Mutex<actix_ws::Session>>, mut msg_stream: actix_ws::MessageStream) {
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

async fn echo_ws(
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
        handle(session, msg_stream).await;
    });

    return Ok(res);
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let client = redis::Client::open("redis://127.0.0.1").unwrap();
    let (mut sink, stream) = client.get_async_pubsub().await.unwrap().split();
    sink.subscribe(format!("worker:{}", REPLICA_ID))
        .await
        .unwrap();

    // pool will be used to publish messages
    let pool: r2d2::Pool<redis::Client> = r2d2::Pool::builder().build(client).unwrap();
    let ws_connections = Arc::new(RwLock::new(WsConnections::new()));

    spawn_redis_worker(stream, ws_connections.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(ws_connections.clone()))
            .service(web::resource("/ws").route(web::get().to(echo_ws)))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
