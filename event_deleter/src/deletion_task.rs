use crate::event_analyzer::DeleteRequest;
use crate::relay_commander::{RawCommanderTrait, RelayCommander};
use std::num::NonZeroU16;
use tokio::sync::mpsc;
use tokio::time;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info};

static FLUSH_PERIOD_SECONDS: u64 = 10;

pub fn spawn_deletion_task<T: RawCommanderTrait>(
    tracker: &TaskTracker,
    mut deletion_receiver: mpsc::Receiver<DeleteRequest>,
    ack_sender: Option<mpsc::Sender<DeleteRequest>>,
    relay_commander: RelayCommander<T>,
    buffer_size: NonZeroU16,
    dry_run: bool,
) {
    tracker.spawn(async move {
        let mut buffer = Vec::with_capacity(buffer_size.get() as usize);
        let flush_period = time::Duration::from_secs(FLUSH_PERIOD_SECONDS);

        info!("Publishing messages every {} seconds", FLUSH_PERIOD_SECONDS);

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

async fn flush_buffer<T: RawCommanderTrait>(
    relay_commander: &RelayCommander<T>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_sdk::prelude::*;
    use std::collections::BTreeSet;
    use std::num::NonZeroU16;
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;
    use tokio::time::{self, Duration};
    use tokio_util::task::TaskTracker;

    #[derive(Debug)]
    struct CommandRun {
        filter: Filter,
        dry_run: bool,
    }

    // MockRelayCommander that records calls to execute_delete
    #[derive(Clone)]
    struct MockRelayCommander {
        executed_deletes: Arc<Mutex<Vec<CommandRun>>>,
    }

    #[async_trait::async_trait]
    impl RawCommanderTrait for MockRelayCommander {
        async fn delete_from_filter(
            &self,
            filter: Filter,
            dry_run: bool,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let command_run = CommandRun { filter, dry_run };
            let mut executed_deletes = self.executed_deletes.lock().unwrap();
            executed_deletes.push(command_run);
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_deletion_task() {
        let buffer_size = NonZeroU16::new(3).unwrap(); // Small buffer size for testing
        let dry_run = false;
        let tracker = TaskTracker::new();
        let (deletion_sender, deletion_receiver) = mpsc::channel(10);
        let (ack_sender, mut ack_receiver) = mpsc::channel(10); // Optional acknowledgment channel
        let executed_deletes = Arc::new(Mutex::new(Vec::new()));
        let mock_commander = MockRelayCommander {
            executed_deletes: executed_deletes.clone(),
        };
        let relay_commander = RelayCommander::new(mock_commander);

        spawn_deletion_task(
            &tracker,
            deletion_receiver,
            Some(ack_sender),
            relay_commander,
            buffer_size,
            dry_run,
        );
        tracker.close();

        // Send DeleteRequests
        let forbidden_public_key = Keys::generate().public_key();
        let forbidden_name = DeleteRequest::ForbiddenName(forbidden_public_key);
        let event_id =
            EventId::parse("ae7603d8af87cb3b055fd6955692e3201cbd42ae1e327e16fc0c32ab5e888d63")
                .unwrap();
        let reply_copy = DeleteRequest::ReplyCopy(event_id);
        let vanish_public_key = Keys::generate().public_key();
        let vanish = DeleteRequest::Vanish("streamid".to_string(), vanish_public_key, None);

        deletion_sender.send(forbidden_name.clone()).await.unwrap();
        deletion_sender.send(reply_copy.clone()).await.unwrap();
        deletion_sender.send(vanish.clone()).await.unwrap();

        // Wait an interval cycle
        time::advance(Duration::from_secs(FLUSH_PERIOD_SECONDS)).await;

        // Check that execute_delete was called with the correct filters
        assert_executed_deletes(
            &executed_deletes,
            dry_run,
            vec![
                CommandExpectation {
                    expected_ids: Some(BTreeSet::from([event_id])),
                    expected_authors: None,
                },
                CommandExpectation {
                    expected_ids: None,
                    expected_authors: Some(BTreeSet::from([
                        forbidden_public_key,
                        vanish_public_key,
                    ])),
                },
            ],
        );

        // Check that acknowledgments were sent
        assert_acks_received(&mut ack_receiver, vec![forbidden_name, reply_copy, vanish]).await;

        drop(deletion_sender);
        tracker.wait().await;
    }

    struct CommandExpectation {
        expected_ids: Option<BTreeSet<EventId>>,
        expected_authors: Option<BTreeSet<PublicKey>>,
    }

    fn assert_executed_deletes(
        executed_deletes: &Arc<Mutex<Vec<CommandRun>>>,
        expected_dry_run: bool,
        expected_commands: Vec<CommandExpectation>,
    ) {
        let executed = executed_deletes.lock().unwrap();
        assert_eq!(executed.len(), expected_commands.len());

        for (command_run, expectation) in executed.iter().zip(expected_commands.iter()) {
            assert_eq!(command_run.dry_run, expected_dry_run);
            assert_eq!(&command_run.filter.ids, &expectation.expected_ids);
            assert_eq!(&command_run.filter.authors, &expectation.expected_authors);
        }
    }

    async fn assert_acks_received(
        ack_receiver: &mut mpsc::Receiver<DeleteRequest>,
        expected_acks: Vec<DeleteRequest>,
    ) {
        let mut acks = Vec::new();
        while let Ok(ack) = ack_receiver.try_recv() {
            acks.push(ack);
        }
        assert_eq!(acks.len(), expected_acks.len());
        for expected_ack in expected_acks {
            assert!(acks.contains(&expected_ack));
        }
    }
}
