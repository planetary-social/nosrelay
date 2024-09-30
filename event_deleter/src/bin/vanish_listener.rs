use clap::Parser;
use event_deleter::{
    deletion_task::spawn_deletion_task, event_analyzer::DeleteRequest,
    vanish_listener_task::spawn_vanish_listener,
};
use nonzero_ext::nonzero;
use std::error::Error;
use std::{env, sync::LazyLock};
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static REDIS_URL: LazyLock<String> =
    LazyLock::new(|| env::var("REDIS_URL").expect("REDIS_URL must be set"));

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Checks events for spam and deletes them from the strfry database",
    long_about = None
)]
// Leave the comments, they are used for the --help message
struct Args {
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
    let token = cancellation_token.clone();

    info!("Starting vanish listener...");
    info!("Dry run: {}", args.dry_run);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        token.cancel();
        info!("Shutdown signal received. Initiating graceful shutdown...");
    });

    // We may never need to change these constants so for the moment lets leave them hardcoded
    let delete_command_batch_size = nonzero!(50u16);
    let vanish_channel_size = 10;
    let (deletion_sender, deletion_receiver) = mpsc::channel::<DeleteRequest>(vanish_channel_size);
    let (ack_sender, ack_receiver) = mpsc::channel::<DeleteRequest>(vanish_channel_size);

    // Read the Redis stream and send the delete requests to the deletion task
    spawn_vanish_listener(
        &tracker,
        deletion_sender,
        ack_receiver,
        &*REDIS_URL,
        cancellation_token,
    )
    .await?;
    spawn_deletion_task(
        &tracker,
        deletion_receiver,
        Some(ack_sender),
        delete_command_batch_size,
        args.dry_run,
    );

    tracker.close();
    tracker.wait().await;

    info!("Exiting vanish listener");

    Ok(())
}
