use std::io;
use std::sync::{Arc, RwLock};

use actix_web::web::{self, Data};
use actix_web::{App, HttpServer};
use api::send_message;
use redis_worker::{generate_replica_id, subscribe_worker};

mod redis_worker;
use crate::redis_worker::spawn_redis_worker;

mod ws;
use crate::ws::WsConnections;

mod api;
use crate::api::echo_ws;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let client = redis::Client::open("redis://redis").unwrap();
    let replica_id;
    // I have put it in separate block because otherwise the connection to redis would never be
    // closed as the main function ends only when the HttpServer closes
    {
        let mut redis_con = client.get_connection().unwrap();
        replica_id = generate_replica_id(&mut redis_con);
    }

    let stream = subscribe_worker(&client, &replica_id).await.unwrap();

    // pool will be used to publish messages
    let pool: r2d2::Pool<redis::Client> = r2d2::Pool::builder().build(client).unwrap();
    println!("this is my id {replica_id}");
    let ws_connections = Arc::new(RwLock::new(WsConnections::new()));

    spawn_redis_worker(stream, ws_connections.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(ws_connections.clone()))
            .app_data(Data::new(replica_id.clone()))
            .service(web::resource("/ws/{client_id}").route(web::get().to(echo_ws)))
            .service(web::resource("/messages/{client_id}").route(web::post().to(send_message)))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
