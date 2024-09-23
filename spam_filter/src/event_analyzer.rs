use nostr_sdk::prelude::*;
use thiserror::Error as ThisError;
use tokio::time::Duration;
use tracing::debug;
use tracing_subscriber::field::debug;

// TODO: get port from args
static LOCAL_RELAY_URL: &str = "ws://localhost:7777";

#[derive(Debug, Clone)]
pub enum EventAnalysisResult {
    Accept(Event),
    Reject(String),
}

#[derive(Clone)]
pub struct Validator {
    nostr_client: Client,
}

impl Validator {
    pub async fn new() -> Result<Self, EventAnalysisError> {
        let opts = Options::default()
            .skip_disconnected_relays(true)
            .wait_for_send(false)
            .connection_timeout(Some(Duration::from_secs(5)))
            .send_timeout(Some(Duration::from_secs(5)))
            .wait_for_subscription(true);

        let nostr_client = ClientBuilder::default().opts(opts).build();
        if let Err(e) = nostr_client.add_relay(LOCAL_RELAY_URL).await {
            return Err(EventAnalysisError::ConnectionError(e));
        }

        nostr_client.connect().await;

        Ok(Validator { nostr_client })
    }

    pub async fn validate_event(
        &self,
        event: Event,
    ) -> Result<EventAnalysisResult, EventAnalysisError> {
        debug!("Start to validating event {}", event.id);
        for event_id in event_ids(&event) {
            let filters = vec![Filter::new().id(*event_id)];

            debug!("Searching for event with filter {:?}", filters);
            let result = self
                .nostr_client
                .get_events_of(filters, EventSource::both(None))
                .await
                .map_err(EventAnalysisError::NostrError)?;

            debug!("Found {} events with id {}", result.len(), event_id);
            let Some(referenced_event) = result.first() else {
                continue;
            };

            // TODO: similarity + frequency?
            debug!(
                "Content referenced: {}, this content: {}",
                referenced_event.content, event.content
            );

            // TODO: add a size limit?
            if referenced_event.content == event.content && event.pubkey == referenced_event.pubkey
            {
                debug!(
                    "Event {} is a copy of event {}",
                    event.id, referenced_event.id
                );
                return Ok(EventAnalysisResult::Reject(
                    "Event copies referenced content".to_string(),
                ));
            }
        }

        Ok(EventAnalysisResult::Accept(event))
    }
}

fn event_ids(event: &Event) -> impl Iterator<Item = &EventId> {
    let tags = event.tags.iter().filter_map(|t| match t.as_standardized() {
        Some(TagStandard::Event { event_id, .. }) => Some(event_id),
        _ => None,
    });

    debug!(
        "Event tags for event {}: {:?}, found {:?}",
        event.id, event.tags, tags
    );
    tags
}

#[derive(ThisError, Debug)]
pub enum EventAnalysisError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(Error),

    #[error("Nostr error: {0}")]
    NostrError(Error),
}
