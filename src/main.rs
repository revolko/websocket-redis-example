use std::io;
use std::sync::{Arc, RwLock};

use actix_web::web::{self, Data};
use actix_web::{App, HttpServer};

mod redis_worker;
use crate::redis_worker::spawn_redis_worker;

mod ws;
use crate::ws::{echo_ws, WsConnections};

const REPLICA_ID: i8 = 1;

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
