use clap::Parser;
use event_deleter::{
    analyzer_worker::ValidationWorker,
    deletion_task::spawn_deletion_task,
    event_analyzer::{DeleteRequest, Validator},
    worker_pool::WorkerPool,
};
use nonzero_ext::nonzero;
use nostr_sdk::Event;
use serde_json::Deserializer;
use std::error::Error;
use std::io;
use std::num::NonZeroU16;
use std::num::NonZeroU64;
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Checks events for spam and deletes them from the strfry database",
    long_about = None
)]
// Leave the comments, they are used for the --help message
struct Args {
    /// Buffer size for batching delete commands
    #[arg(short, long, default_value_t = nonzero!(10u16))]
    buffer_size: NonZeroU16,

    /// Maximum number of concurrent validation tasks
    #[arg(short = 'c', long, default_value_t = nonzero!(10u16))]
    concurrency_limit: NonZeroU16,

    /// Timeout (in seconds) for validating each event
    #[arg(short = 't', long, default_value_t = nonzero!(10u64))]
    validation_timeout: NonZeroU64,

    /// Dry run mode. If set, events will not be deleted
    #[arg(short = 'd', long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let args = Args::parse();
    let tracker = TaskTracker::new();
    let cancellation_token = CancellationToken::new();
    let shutdown_token = cancellation_token.clone();

    info!("Starting spam cleaner...");
    info!(
        "Buffer size: {}, Concurrency limit: {}, Validation timeout: {}, Dry run: {}",
        args.buffer_size, args.concurrency_limit, args.validation_timeout, args.dry_run
    );

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        shutdown_token.cancel();
        info!("Shutdown signal received. Initiating graceful shutdown...");
    });

    let (validation_sender, validation_receiver) = mpsc::channel::<Event>(100);
    let (deletion_sender, deletion_receiver) = mpsc::channel::<DeleteRequest>(100);

    let validator = Validator::new().await?;
    let validator_worker =
        ValidationWorker::new(validator, deletion_sender, args.validation_timeout);

    // Spawn the validation WorkerPool
    WorkerPool::start(
        &tracker,
        "validation_pool",
        args.concurrency_limit,
        args.validation_timeout,
        validation_receiver,
        cancellation_token.clone(),
        validator_worker,
    );

    // Spawn the deletion task with dry_run flag
    spawn_deletion_task(
        &tracker,
        deletion_receiver,
        None,
        args.buffer_size,
        args.dry_run,
    );

    tracker.close();

    // Read events from stdin as a JSONL stream
    let stdin = io::stdin();
    let reader = stdin.lock();
    let deserializer = Deserializer::from_reader(reader).into_iter::<Event>();

    debug!("Reading events from stdin...");

    for event in deserializer {
        if cancellation_token.is_cancelled() {
            debug!("Cancellation token is cancelled. Stopping event reader...");
            break;
        }

        debug!("Received event: {:?}", event);
        match event {
            Ok(ev) => {
                if let Err(e) = validation_sender.send(ev).await {
                    error!("Failed to send event to validation pool: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Failed to parse event: {}", e);
            }
        }
    }

    debug!("Finished reading events from stdin. Flushing...");

    drop(validation_sender);

    tracker.wait().await;

    debug!("Exiting main function");

    Ok(())
}
