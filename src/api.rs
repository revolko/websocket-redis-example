use std::sync::{Arc, RwLock};

use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use redis::Commands;

use super::WsConnections;
use crate::redis_worker::{register_client, ClientMessage};
use crate::ws::handle_ws;
use crate::REPLICA_ID;

pub async fn echo_ws(
    req: HttpRequest,
    client_id: web::Path<String>,
    body: web::Payload,
    ws_connections: web::Data<Arc<RwLock<WsConnections>>>,
    redis_pool: web::Data<r2d2::Pool<redis::Client>>,
) -> actix_web::Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, body)?;
    let client_id = client_id.into_inner();
    let ws_connections = ws_connections.into_inner();
    let mut redis_con = redis_pool.into_inner().get().unwrap();

    actix_web::rt::spawn(async move {
        println!("connected {}", client_id);
        let session = ws_connections
            .write()
            .unwrap()
            .add_session(&client_id, session);
        register_client(&mut redis_con, &REPLICA_ID, &client_id).unwrap();
        handle_ws(session, msg_stream).await;
    });

    return Ok(res);
}

pub async fn send_message(
    client_id: web::Path<String>,
    json_body: web::Json<String>,
    redis_pool: web::Data<r2d2::Pool<redis::Client>>,
) -> actix_web::Result<HttpResponse, Error> {
    let mut redis_con = redis_pool.into_inner().get().unwrap();
    let client_id = client_id.into_inner();
    let msg = json_body.into_inner();
    let client_message = ClientMessage {
        client_id: client_id.clone(),
        msg,
    };

    let msg_serialize = serde_json::to_string(&client_message).unwrap();
    let worker_id: i8 = redis_con.hget("clients:connection", client_id).unwrap();
    let channel = format!("worker:{}", worker_id);

    let _: () = redis_con.publish(channel, &msg_serialize).unwrap();
    return Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .status(StatusCode::CREATED)
        .body(msg_serialize));
}
