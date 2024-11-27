use std::sync::{Arc, RwLock};

use actix_web::{web, Error, HttpRequest, HttpResponse};

use super::WsConnections;
use crate::ws::handle_ws;

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
