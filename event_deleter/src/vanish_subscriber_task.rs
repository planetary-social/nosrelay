use crate::event_analyzer::DeleteRequest;
use async_trait::async_trait;
use redis::{
    streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply},
    AsyncCommands, RedisError,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{debug, error, info};

static BLOCK_MILLIS: usize = 5000;
static VANISH_STREAM_KEY: &str = "vanish_requests";
static VANISH_LAST_ID_KEY: &str = "vanish_requests:deletion_subscriber:last_id";

pub struct RedisClient {
    client: redis::Client,
}

#[async_trait]
pub trait RedisClientTrait: Send + Sync + 'static {
    type Connection: RedisClientConnectionTrait;
    async fn get_multiplexed_async_connection(&self) -> Result<Self::Connection, RedisError>;
}

impl RedisClient {
    pub fn new(url: &str) -> Self {
        let client = redis::Client::open(url).expect("Failed to create Redis client");
        RedisClient { client }
    }
}

#[async_trait]
impl RedisClientTrait for RedisClient {
    type Connection = RedisClientConnection;
    async fn get_multiplexed_async_connection(&self) -> Result<Self::Connection, RedisError> {
        let con = self.client.get_multiplexed_async_connection().await?;
        Ok(RedisClientConnection { con })
    }
}

pub struct RedisClientConnection {
    con: redis::aio::MultiplexedConnection,
}

#[async_trait]
pub trait RedisClientConnectionTrait: Send + Sync + 'static {
    async fn get(&mut self, key: &str) -> Result<String, RedisError>;
    async fn set(&mut self, key: &str, value: String) -> Result<(), RedisError>;
    async fn xread_options(
        &mut self,
        keys: &[&str],
        ids: &[String],
        opts: &StreamReadOptions,
    ) -> Result<StreamReadReply, RedisError>;
}

#[async_trait]
impl RedisClientConnectionTrait for RedisClientConnection {
    async fn get(&mut self, key: &str) -> Result<String, RedisError> {
        self.con.get(key).await
    }

    async fn set(&mut self, key: &str, value: String) -> Result<(), RedisError> {
        self.con.set(key, value).await
    }

    async fn xread_options(
        &mut self,
        keys: &[&str],
        ids: &[String],
        opts: &StreamReadOptions,
    ) -> Result<StreamReadReply, RedisError> {
        self.con.xread_options(keys, ids, opts).await
    }
}

pub async fn spawn_vanish_subscriber<T: RedisClientTrait>(
    tracker: &TaskTracker,
    deletion_sender: mpsc::Sender<DeleteRequest>,
    mut ack_receiver: mpsc::Receiver<DeleteRequest>,
    redis_client: T,
    cancellation_token: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    let redis_client = Arc::new(redis_client);

    let redis_client_clone = redis_client.clone();
    tracker.spawn(async move {
        let (mut con, mut last_id) = match get_connection_and_last_id(redis_client_clone).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                return;
            }
        };

        while let Some(ack) = ack_receiver.recv().await {
            if let DeleteRequest::Vanish(id, ..) = ack {
                debug!("Received ack");

                if id > last_id {
                    let save_last_id_result: Result<(), RedisError> =
                        con.set(VANISH_LAST_ID_KEY, last_id.clone()).await;

                    if let Err(e) = save_last_id_result {
                        error!("Failed to save last id: {}", e);
                    } else {
                        info!("Updating last vanish stream id processed to {}", last_id);
                    }

                    last_id = id.clone();
                }
            }
        }
    });

    let redis_client_clone = redis_client.clone();
    tracker.spawn(async move {
        let (mut con, mut last_id) = match get_connection_and_last_id(redis_client_clone).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                return;
            }
        };

        let opts = StreamReadOptions::default().block(BLOCK_MILLIS);

        info!("Starting from last id processed: {}", last_id);

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }

                result = async {
                    let reply: StreamReadReply = con
                        .xread_options(&[VANISH_STREAM_KEY], &[last_id.clone()], &opts)
                        .await?;

                    for StreamKey { ids, .. } in reply.keys {
                        for stream_id in ids {
                            if stream_id.id == last_id {
                                continue;
                            }

                            process_stream_id(&stream_id, &deletion_sender).await?;
                            last_id = stream_id.id.clone();
                        }
                    }
                    Ok::<(), Box<dyn Error>>(())
                } => {
                    if let Err(e) = result {
                        error!("Error in Redis stream reader task: {}", e);
                        continue;
                    }
                }
            }
        }
    });

    Ok(())
}

async fn get_connection_and_last_id<T: RedisClientTrait>(
    redis_client: Arc<T>,
) -> Result<(T::Connection, String), RedisError> {
    let mut con = redis_client.get_multiplexed_async_connection().await?;
    let last_id = con
        .get(VANISH_LAST_ID_KEY)
        .await
        .unwrap_or_else(|_| "0-0".to_string());
    Ok((con, last_id))
}

async fn process_stream_id(
    stream_id: &StreamId,
    deletion_sender: &mpsc::Sender<DeleteRequest>,
) -> Result<(), Box<dyn Error>> {
    let vanish_request = match DeleteRequest::try_from(stream_id) {
        Ok(vanish_request) => vanish_request,
        Err(e) => {
            // Log the error and continue processing the next stream id
            error!(
                "Couldn't process vanish request: {:?}. Error: {}",
                stream_id, e
            );

            return Ok(());
        }
    };

    info!("Received vanish request: {:?}", vanish_request);

    deletion_sender.send(vanish_request).await.map_err(|e| {
        error!("Failed to send vanish request: {}", e);
        e
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::Keys;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct MockRedisClient {
        last_id: String,
        stream_ids_sequence: Arc<Mutex<Vec<StreamReadReply>>>,
    }
    struct MockRedisClientConnection {
        last_id: String,
        stream_ids_sequence: Arc<Mutex<Vec<StreamReadReply>>>,
        index: usize,
    }

    #[async_trait::async_trait]
    impl RedisClientConnectionTrait for MockRedisClientConnection {
        async fn get(&mut self, _key: &str) -> Result<String, RedisError> {
            Ok(self.last_id.clone())
        }

        async fn set(&mut self, _key: &str, value: String) -> Result<(), RedisError> {
            self.last_id = value;
            Ok(())
        }

        async fn xread_options(
            &mut self,
            _keys: &[&str],
            _ids: &[String],
            _opts: &StreamReadOptions,
        ) -> Result<StreamReadReply, RedisError> {
            tokio::task::yield_now().await;

            let sequence = self.stream_ids_sequence.lock().unwrap();

            if self.index < sequence.len() {
                let reply = sequence[self.index].clone();
                self.index += 1;
                Ok(reply)
            } else {
                Ok(StreamReadReply { keys: Vec::new() })
            }
        }
    }

    impl MockRedisClient {
        fn new(last_id: String, stream_ids_sequence: Arc<Mutex<Vec<StreamReadReply>>>) -> Self {
            MockRedisClient {
                last_id,
                stream_ids_sequence,
            }
        }
    }

    #[async_trait::async_trait]
    impl RedisClientTrait for MockRedisClient {
        type Connection = MockRedisClientConnection;
        async fn get_multiplexed_async_connection(&self) -> Result<Self::Connection, RedisError> {
            Ok(MockRedisClientConnection {
                last_id: self.last_id.clone(),
                stream_ids_sequence: self.stream_ids_sequence.clone(),
                index: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_spawn_vanish_subscriber() {
        let expected_public_key_1 = Keys::generate().public_key();
        let expected_public_key_2 = Keys::generate().public_key();

        let stream_read_reply_1 = StreamReadReply {
            keys: vec![StreamKey {
                key: VANISH_STREAM_KEY.to_string(),
                ids: vec![StreamId {
                    id: "1-0".to_string(),
                    map: HashMap::from([
                        (
                            "pubkey".to_string(),
                            redis::Value::BulkString(expected_public_key_1.to_hex().into()),
                        ),
                        ("kind".to_string(), redis::Value::Int(62)),
                        (
                            "content".to_string(),
                            redis::Value::BulkString("First message".into()),
                        ),
                        (
                            "tags".to_string(),
                            redis::Value::BulkString("all_relays".into()),
                        ),
                    ]),
                }],
            }],
        };

        let stream_read_reply_2 = StreamReadReply {
            keys: vec![StreamKey {
                key: VANISH_STREAM_KEY.to_string(),
                ids: vec![StreamId {
                    id: "2-0".to_string(),
                    map: HashMap::from([
                        (
                            "pubkey".to_string(),
                            redis::Value::BulkString(expected_public_key_2.to_hex().into()),
                        ),
                        ("kind".to_string(), redis::Value::Int(62)),
                        (
                            "content".to_string(),
                            redis::Value::BulkString("Second message".into()),
                        ),
                        (
                            "tags".to_string(),
                            redis::Value::BulkString("all_relays".into()),
                        ),
                    ]),
                }],
            }],
        };

        let stream_ids_sequence =
            Arc::new(Mutex::new(vec![stream_read_reply_1, stream_read_reply_2]));

        let redis_client = MockRedisClient::new("0-0".to_string(), stream_ids_sequence.clone());
        let (deletion_sender, mut deletion_receiver) = mpsc::channel::<DeleteRequest>(10);
        let (ack_sender, ack_receiver) = mpsc::channel(10);
        let cancellation_token = CancellationToken::new();
        let tracker = TaskTracker::new();

        let received_requests = Arc::new(Mutex::new(Vec::new()));
        let received_requests_clone = Arc::clone(&received_requests);
        let token = cancellation_token.clone();

        // Faked deletion task
        let len = stream_ids_sequence.lock().unwrap().len();
        tracker.spawn(async move {
            for _ in 0..len {
                let request = deletion_receiver.recv().await.unwrap();
                received_requests_clone
                    .lock()
                    .unwrap()
                    .push(request.clone());
                ack_sender.send(request).await.unwrap();
            }

            token.cancel();
        });

        spawn_vanish_subscriber(
            &tracker,
            deletion_sender,
            ack_receiver,
            redis_client,
            cancellation_token,
        )
        .await
        .unwrap();
        tracker.close();
        tracker.wait().await;

        let requests = received_requests.lock().unwrap();
        assert_eq!(requests.len(), 2);

        if let DeleteRequest::Vanish(id, public_key, reason) = &requests[0] {
            assert_eq!(id, "1-0");
            assert_eq!(*public_key, expected_public_key_1);
            assert_eq!(reason, &Some("First message".to_string()));
        } else {
            panic!("Expected first request to be Vanish");
        }

        if let DeleteRequest::Vanish(id, public_key, reason) = &requests[1] {
            assert_eq!(id, "2-0");
            assert_eq!(*public_key, expected_public_key_2);
            assert_eq!(reason, &Some("Second message".to_string()));
        } else {
            panic!("Expected second request to be Vanish");
        }
    }
}
