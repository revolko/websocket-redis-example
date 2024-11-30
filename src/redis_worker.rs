use crate::WsConnections;

use futures_util::StreamExt;
use redis::aio::PubSubStream;
use redis::{self, Commands, Connection, RedisResult};
use uuid::Uuid;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

pub const CONNECTIONS_MAP: &str = "clients:connection";
pub const CHANNEL: &str = "worker";

#[derive(Serialize, Deserialize)]
pub struct ClientMessage {
    pub client_id: String,
    pub msg: String,
}

/// Register client to the server replica
pub fn register_client(
    redis_con: &mut Connection,
    replica_id: &str,
    client_id: &str,
) -> RedisResult<()> {
    let _: () = redis_con.hset(CONNECTIONS_MAP, client_id, replica_id)?;
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

pub fn get_worker_channel(replica_id: &str) -> String {
    return format!("{CHANNEL}:{replica_id}");
}

pub async fn subscribe_worker(
    client: &redis::Client,
    replica_id: &str,
) -> RedisResult<PubSubStream> {
    let (mut sink, stream) = client.get_async_pubsub().await?.split();
    sink.subscribe(get_worker_channel(replica_id))
        .await
        .unwrap();

    return Ok(stream);
}

pub fn generate_replica_id(redis_con: &mut Connection) -> String {
    let existing_ids: HashSet<String> = redis::cmd("PUBSUB")
        .arg("CHANNELS")
        .clone()
        .iter(redis_con)
        .unwrap()
        .collect();

    let mut replica_id = Uuid::new_v4().to_string();
    while existing_ids.contains(&replica_id) {
        replica_id = Uuid::new_v4().to_string();
    }

    return replica_id;
}
