use crate::event_analyzer::DeleteRequest;
use crate::relay_commander::RelayCommander;
use std::num::NonZeroU16;
use tokio::sync::mpsc;
use tokio::time;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info};

pub fn spawn_deletion_task(
    tracker: &TaskTracker,
    mut deletion_receiver: mpsc::Receiver<DeleteRequest>,
    ack_sender: Option<mpsc::Sender<DeleteRequest>>,
    buffer_size: NonZeroU16,
    dry_run: bool,
) {
    let relay_commander = RelayCommander;

    tracker.spawn(async move {
        let mut buffer = Vec::with_capacity(buffer_size.get() as usize);
        let flush_period_seconds = 30;
        let flush_period = time::Duration::from_secs(flush_period_seconds);

        info!("Publishing messages every {} seconds", flush_period_seconds);

        let mut interval = time::interval(flush_period);

        loop {
            tokio::select! {
                // The first condition to send the current buffer is the
                // time interval. We wait a max of `flush_period_seconds`
                // seconds, after that the buffer is cleared and sent
                _ = interval.tick() => {
                    flush_buffer(&relay_commander, &mut buffer, &ack_sender, dry_run).await;
                }

                recv_result = deletion_receiver.recv() => {
                    match recv_result {
                        Some(reject_reason) => {
                            buffer.push(reject_reason);

                            // We also check if the buffer is full based on the buffer size
                            if buffer.len() >= buffer_size.get() as usize {
                                flush_buffer(&relay_commander, &mut buffer, &ack_sender, dry_run).await;
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }

        // Flush any pending items before exiting
        flush_buffer(&relay_commander, &mut buffer, &ack_sender, dry_run).await;
        debug!("Deletion task finished");
    });
}

async fn flush_buffer(
    relay_commander: &RelayCommander,
    buffer: &mut Vec<DeleteRequest>,
    ack_sender: &Option<mpsc::Sender<DeleteRequest>>,
    dry_run: bool,
) {
    debug!("Flushing delete command buffer, {} items", buffer.len());

    if !buffer.is_empty() {
        let chunk_clone = buffer.clone();
        let chunk = std::mem::take(buffer);

        if let Err(e) = relay_commander.execute_delete(chunk, dry_run).await {
            error!("{}", e);
        }

        if let Some(ack_sender) = ack_sender {
            for item in chunk_clone {
                if let Err(e) = ack_sender.send(item).await {
                    error!("{}", e);
                }
            }
        }
    }
}
