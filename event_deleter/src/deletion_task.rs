use crate::event_analyzer::RejectReason;
use crate::relay_commander::RelayCommander;
use tokio::sync::mpsc;
use tokio::time;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info};

pub fn spawn_deletion_task(
    tracker: &TaskTracker,
    mut deletion_receiver: mpsc::Receiver<RejectReason>,
    buffer_size: usize,
    dry_run: bool,
) {
    let relay_commander = RelayCommander;

    tracker.spawn(async move {
        let mut buffer = Vec::with_capacity(buffer_size);
        let flush_period_seconds = 30;
        let flush_period = time::Duration::from_secs(flush_period_seconds);

        info!("Publishing messages every {} seconds", flush_period_seconds);

        let mut interval = time::interval(flush_period);

        loop {
            tokio::select! {
                // The first condition to send the current buffer is the
                // time interval. We wait a max of `seconds_threshold`
                // seconds, after that the buffer is cleared and sent
                _ = interval.tick() => {
                    flush_buffer(&relay_commander, &mut buffer, dry_run).await;
                }

                recv_result = deletion_receiver.recv() => {
                    match recv_result {
                        Some(reject_reason) => {
                            buffer.push(reject_reason);
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }

        flush_buffer(&relay_commander, &mut buffer, dry_run).await;
        debug!("Deletion task finished");
    });
}

async fn flush_buffer(
    relay_commander: &RelayCommander,
    buffer: &mut Vec<RejectReason>,
    dry_run: bool,
) {
    debug!(
        "Time based threshold elapsed, publishing buffer, {} items",
        buffer.len()
    );

    if !buffer.is_empty() {
        let chunk = std::mem::take(buffer);
        if let Err(e) = relay_commander.execute_delete(chunk, dry_run).await {
            error!("{}", e);
        }
    }
}
