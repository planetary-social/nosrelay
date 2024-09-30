use crate::event_analyzer::DeleteRequest;
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
static VANISH_LAST_ID_KEY: &str = "vanish_listener:last_id";

pub async fn spawn_vanish_listener(
    tracker: &TaskTracker,
    deletion_sender: mpsc::Sender<DeleteRequest>,
    mut ack_receiver: mpsc::Receiver<DeleteRequest>,
    redis_url: &str,
    cancellation_token: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    let redis = Arc::new(redis::Client::open(redis_url)?);

    let client = redis.clone();
    tracker.spawn(async move {
        let mut con = match client.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(e) => {
                error!("Failed to connect to Redis: {}", e);
                return;
            }
        };

        let mut last_id = con
            .get(VANISH_LAST_ID_KEY)
            .await
            .unwrap_or("0-0".to_string());

        loop {
            match ack_receiver.recv().await {
                Some(ack) => match ack {
                    DeleteRequest::Vanish(id, ..) => {
                        debug!("Received ack");

                        if id > last_id {
                            let save_last_id_result: Result<(), RedisError> =
                                con.set(VANISH_LAST_ID_KEY, last_id.clone()).await;

                            if let Err(e) = save_last_id_result {
                                error!("Failed to save last id: {}", e);
                            } else {
                                info!("Last id processed: {}", last_id);
                            }
                            last_id = id.clone();
                        }
                    }
                    _ => {}
                },
                None => {
                    break;
                }
            }
        }
    });

    let client = redis.clone();
    tracker.spawn(async move {
        let mut con = match client.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(e) => {
                error!("Failed to connect to Redis: {}", e);
                return;
            }
        };

        let opts = StreamReadOptions::default().block(BLOCK_MILLIS);
        let mut last_id = con
            .get(VANISH_LAST_ID_KEY)
            .await
            .unwrap_or("0-0".to_string());

        info!("Last id processed: {}", last_id);

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    break;
                }

                _ = async {
                    let reply: StreamReadReply = match con
                        .xread_options(&[VANISH_STREAM_KEY], &[last_id.clone()], &opts)
                        .await
                    {
                        Ok(reply) => reply,
                        Err(e) => {
                            error!("Failed to read from Redis: {}", e);
                            return;
                        }
                    };

                    for StreamKey { ids, .. } in reply.keys {
                        for stream_id in ids {
                            if let Err(_) = process_stream_id(&stream_id, &deletion_sender).await {
                                return;
                            }
                            last_id = stream_id.id.clone();
                        }
                    }
                } => {}
            }
        }
    });

    Ok(())
}

async fn process_stream_id(
    stream_id: &StreamId,
    deletion_sender: &mpsc::Sender<DeleteRequest>,
) -> Result<(), Box<dyn Error>> {
    let vanish_request = DeleteRequest::try_from(stream_id).map_err(|e| {
        error!(
            "Failed to parse vanish request: {:?}. Error: {}",
            stream_id, e
        );
        e
    })?;

    info!("Received vanish request: {:?}", vanish_request);

    deletion_sender.send(vanish_request).await.map_err(|e| {
        error!("Failed to send vanish request: {}", e);
        e
    })?;

    Ok(())
}
