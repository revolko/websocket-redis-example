use crate::WsConnections;

use futures_util::StreamExt;
use redis::aio::PubSubStream;
use redis::{self, Commands, Connection, RedisResult};

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClientMessage {
    pub client_id: String,
    pub msg: String,
}

/// Register client to the server replica
pub fn register_client(
    redis_con: &mut Connection,
    replica_id: &i8,
    client_id: &str,
) -> RedisResult<()> {
    let _: () = redis_con.hset("clients:connection", client_id, replica_id)?;
    return Ok(());
}

/// Spawn thread handling incoming Redis messages
pub fn spawn_redis_worker(mut stream: PubSubStream, ws_connections: Arc<RwLock<WsConnections>>) {
    actix_web::rt::spawn(async move {
        loop {
            let msg = stream.next().await.unwrap();
            let payload: String = msg.get_payload().unwrap();
            println!("channel '{}': {}", msg.get_channel_name(), payload);
            let client_message: ClientMessage = serde_json::from_str(&payload).unwrap();
            let ws_read_locked = ws_connections.read().unwrap();
            let session = ws_read_locked
                .sessions
                .get(&client_message.client_id)
                .unwrap();
            session
                .lock()
                .unwrap()
                .text(client_message.msg)
                .await
                .unwrap();
        }
    });
}
