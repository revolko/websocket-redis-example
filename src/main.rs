use redis::{self, Connection, PubSub};
use redis::{Commands, RedisResult};

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
    pubsub.subscribe(format!("worker:{}", replica_id))?;

    return Ok(pubsub);
}

fn main() {
    const REPLICA_ID: i8 = 1;
    let client = redis::Client::open("redis://127.0.0.1").unwrap();
    let mut con = client.get_connection().unwrap();

    register_client(&mut con, &REPLICA_ID, "clientID").unwrap();
    let mut queue = initialize_queue(&mut con, &REPLICA_ID).unwrap();

    loop {
        let msg = queue.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();
        println!("channel '{}': {}", msg.get_channel_name(), payload);
    }
}
