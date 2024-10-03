use crate::event_analyzer::{DeleteRequest, EventAnalysisResult, Validator};
use crate::worker_pool::WorkerTask;
use async_trait::async_trait;
use nostr_sdk::prelude::*;
use std::num::NonZeroU64;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{debug, error, info};

pub struct ValidationWorker {
    validator: Validator,
    deletion_sender: mpsc::Sender<DeleteRequest>,
    validation_timeout: NonZeroU64,
}

impl ValidationWorker {
    pub fn new(
        validator: Validator,
        deletion_sender: mpsc::Sender<DeleteRequest>,
        validation_timeout: NonZeroU64,
    ) -> Self {
        ValidationWorker {
            validator,
            deletion_sender,
            validation_timeout,
        }
    }
}

#[async_trait]
impl WorkerTask<Event> for ValidationWorker {
    async fn call(&self, event: Event) -> Result<()> {
        debug!("Validating event {}", event.id);

        match tokio::time::timeout(
            Duration::from_secs(self.validation_timeout.get()),
            self.validator.validate_event(event.clone()),
        )
        .await
        {
            Ok(Ok(EventAnalysisResult::Reject(reason))) => {
                info!("Rejected event {}: {}", event.id, reason);

                if self.deletion_sender.send(reason).await.is_err() {
                    return Err(ValidatorError::ReceiverDropped(event.id).into());
                }
            }
            Ok(Ok(EventAnalysisResult::Accept)) => {
                debug!("Accepted event {}", event.id);
            }
            Ok(Err(e)) => {
                return Err(ValidatorError::ValidationError(e.to_string()).into());
            }
            Err(_) => {
                return Err(ValidatorError::ValidationTimeout.into());
            }
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ValidatorError {
    #[error("Receiver dropped while sending deletion request for event: {0}")]
    ReceiverDropped(EventId),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Validation timed out")]
    ValidationTimeout,
}
