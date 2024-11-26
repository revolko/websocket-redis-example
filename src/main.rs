use std::io;
use std::sync::{Arc, Mutex};

use actix_web::web::{self, Data};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::Message;
use redis::{self, Connection, PubSub};
use redis::{Commands, RedisResult};
use tokio::runtime::Handle;

const REPLICA_ID: i8 = 1;

/// Register client to the server replica
fn register_client(
    redis_con: &mut Connection,
    replica_id: &i8,
    client_id: &str,
) -> RedisResult<()> {
    let _: () = redis_con.hset("clients:connection", client_id, replica_id)?;
    return Ok(());
}

/// Initialize pubsub queue for the replica
fn initialize_queue<'a>(redis_con: &'a mut Connection, replica_id: &i8) -> RedisResult<PubSub<'a>> {
    let mut pubsub = redis_con.as_pubsub();
    let _ = pubsub.subscribe(format!("worker:{}", replica_id));
    return Ok(pubsub);
}

struct WsConnection {
    session: Mutex<actix_ws::Session>,
    msg_stream: Mutex<actix_ws::MessageStream>,
}

async fn handle(
    session: Arc<Mutex<actix_ws::Session>>,
    msg_stream: Arc<Mutex<actix_ws::MessageStream>>,
) {
    while let Some(msg) = msg_stream.lock().unwrap().recv().await {
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
    redis_pool: web::Data<r2d2::Pool<redis::Client>>,
) -> actix_web::Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, body)?;
    let mut redis_con = redis_pool.get().unwrap(); // map to error

    let session = Arc::new(Mutex::new(session));
    let msg_stream = Arc::new(Mutex::new(msg_stream));
    let redis_session = session.clone();
    actix_web::rt::spawn(async move {
        println!("connected");
        handle(session.clone(), msg_stream.clone()).await;
    });

    actix_web::rt::task::spawn_blocking(move || {
        let mut queue = initialize_queue(&mut redis_con, &REPLICA_ID).unwrap();
        let tokio_handle = Handle::current();
        loop {
            let redis_session = redis_session.clone();
            let msg = queue.get_message().unwrap();
            let payload: String = msg.get_payload().unwrap();
            println!("channel '{}': {}", msg.get_channel_name(), payload);
            tokio_handle
                .block_on(redis_session.clone().lock().unwrap().text(payload))
                .unwrap();
            println!("send a message");
        }
    });

    return Ok(res);
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let client = redis::Client::open("redis://127.0.0.1").unwrap();
    let pool: r2d2::Pool<redis::Client> = r2d2::Pool::builder().build(client).unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .service(web::resource("/ws").route(web::get().to(echo_ws)))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
