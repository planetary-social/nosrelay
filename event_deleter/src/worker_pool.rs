use std::fmt::Debug;
use std::sync::Arc;
use std::{error::Error, num::NonZeroUsize};
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, trace};

pub struct WorkerPool {}

// A channel based worker pool that distributes work to a pool of workers.
// Items come through the item_rx channel and are distributed to workers.
// Workers implement the WorkerTask trait that receives the item to process.
impl WorkerPool {
    pub fn start<Item, Worker>(
        tracker: &TaskTracker,
        pool_name: &str,
        num_workers: NonZeroUsize,
        worker_timeout_secs: NonZeroUsize,
        item_receiver: mpsc::Receiver<Item>,
        cancellation_token: CancellationToken,
        worker: Worker,
    ) -> TaskTracker
    where
        Item: Debug + Send + Sync + Clone + 'static,
        Worker: WorkerTask<Item> + Send + Sync + 'static,
    {
        // Spawn a pool of worker tasks to process each item. Call the worker_fn for each item.
        let mut worker_txs = Vec::new();

        let shared_worker = Arc::new(worker);

        for i in 0..num_workers.get() {
            let (worker_tx, worker_rx) = mpsc::channel::<Item>(1);
            worker_txs.push(worker_tx);

            create_worker_task(
                tracker,
                pool_name.to_string(),
                worker_timeout_secs,
                worker_rx,
                shared_worker.clone(),
                i,
            );
        }

        create_dispatcher_task(
            tracker,
            pool_name.to_string(),
            item_receiver,
            worker_txs,
            cancellation_token,
        );

        tracker.clone()
    }
}

fn create_dispatcher_task<Item>(
    tracker: &TaskTracker,
    pool_name: String,
    mut item_receiver: mpsc::Receiver<Item>,
    worker_txs: Vec<Sender<Item>>,
    cancellation_token: CancellationToken,
) where
    Item: Debug + Send + Clone + 'static,
{
    tracker.spawn(async move {
        // Simple cycle iterator to distribute work to workers in a round-robin fashion.
        let mut worker_txs_cycle = worker_txs.iter().cycle();

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    info!("{}: Cancellation token is cancelled, stopping worker pool", pool_name);
                    break;
                }

                result = item_receiver.recv() => {
                    match result {
                        Some(item) => {
                            trace!("{}: Worker pool dispatching item {:?}", pool_name, item);
                            let Some(worker_tx) = worker_txs_cycle.next() else {
                                error!("{}: Failed to get worker", pool_name);
                                break;
                            };

                            if let Err(e) = worker_tx.send(item).await {
                                error!("{}: Failed to send to worker: {}", pool_name, e);
                                break;
                            }
                        }
                        None => {
                            debug!("{}: Item receiver channel closed", pool_name);
                            break;
                        }
                    }
                }
            }
        }

        for worker_tx in worker_txs {
            drop(worker_tx);
        }

        debug!("{}: Worker pool dispatcher finished", pool_name);
    });
}

fn create_worker_task<Item, Worker>(
    tracker: &TaskTracker,
    pool_name: String,
    worker_timeout_secs: NonZeroUsize,
    mut worker_rx: mpsc::Receiver<Item>,
    worker: Arc<Worker>,
    worker_index: usize,
) where
    Item: Debug + Send + Sync + Clone + 'static,
    Worker: WorkerTask<Item> + Send + Sync + 'static,
{
    let worker_name = format!("{}-{}", pool_name, worker_index);
    tracker.spawn(async move {
        loop {
            tokio::select! {
                result = worker_rx.recv() => {
                    match result {
                        None => {
                            debug!("{}: Worker task finished processing all items", worker_name);
                            break;
                        }
                        Some(item) => {
                            trace!("{}: Worker task processing item {:?}", worker_name, item);
                            let result = timeout(Duration::from_secs(worker_timeout_secs.get() as u64), worker.call(item)).await;

                            match result {
                                Ok(Ok(())) => {
                                    trace!("{}: Worker task finished successfully processing item", worker_name);
                                }
                                Ok(Err(e)) => {
                                    error!("{}: Worker failed: {}", worker_name, e);
                                }
                                Err(_) => {
                                    error!("{}: Worker task timed out after {} seconds", worker_name, worker_timeout_secs);
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!("{}: Worker task finished", worker_name);
    });
}

/// The worker task trait that workers must implement to process items.
/// The same instance will be used for all items so common global state can be shared.
#[async_trait::async_trait]
pub trait WorkerTask<Item>
where
    Item: Debug + Send + Sync + Clone + 'static,
{
    async fn call(&self, args: Item) -> Result<(), Box<dyn Error>>;
}
