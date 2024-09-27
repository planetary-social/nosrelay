use nostr_sdk::prelude::*;
use regex::Regex;
use std::fmt::Display;
use std::sync::LazyLock;
use thiserror::Error as ThisError;
use tokio::time::Duration;
use tracing::debug;

// TODO: get port from args
static LOCAL_RELAY_URL: &str = "ws://localhost:7777";

static REJECTED_NAME_REGEXES: LazyLock<Vec<Regex>> =
    LazyLock::new(|| vec![Regex::new(r".*Reply.*(Guy|Girl|Gal).*").unwrap()]);

#[derive(Debug, Clone)]
pub enum EventAnalysisResult {
    Accept,
    Reject(RejectReason),
}

#[derive(Debug, Clone)]
pub enum RejectReason {
    ReplyCopy(EventId),
    ForbiddenName(PublicKey),
}

impl Display for RejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectReason::ReplyCopy(_) => write!(f, "Reply copy"),
            RejectReason::ForbiddenName(_) => write!(f, "Forbidden nip05"),
        }
    }
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

        let (is_reply_copy_res, is_forbidden_name_res) =
            tokio::join!(self.is_reply_copy(&event), self.is_forbidden_name(&event));

        let is_reply_copy = is_reply_copy_res?;
        let is_forbidden_name = is_forbidden_name_res?;

        if is_reply_copy {
            return Ok(EventAnalysisResult::Reject(RejectReason::ReplyCopy(
                event.id,
            )));
        }

        if is_forbidden_name {
            return Ok(EventAnalysisResult::Reject(RejectReason::ForbiddenName(
                event.pubkey,
            )));
        }

        Ok(EventAnalysisResult::Accept)
    }

    async fn is_forbidden_name(&self, event: &Event) -> Result<bool, EventAnalysisError> {
        let filters: Vec<Filter> = vec![Filter::new()
            .author(event.pubkey)
            .kind(Kind::Metadata)
            .limit(1)];

        let Ok(mut events) = self
            .nostr_client
            .get_events_of(filters, EventSource::both(None))
            .await
        else {
            return Ok(false);
        };

        let Some(metadata_event) = events.pop() else {
            return Ok(false);
        };

        let Ok(metadata) = Metadata::from_json(metadata_event.content) else {
            return Ok(false);
        };

        let forbidden = [metadata.nip05, metadata.name, metadata.display_name]
            .iter()
            .any(|name| {
                if let Some(name) = name {
                    REJECTED_NAME_REGEXES.iter().any(|re| re.is_match(name))
                } else {
                    false
                }
            });

        Ok(forbidden)
    }

    async fn is_reply_copy(&self, event: &Event) -> Result<bool, EventAnalysisError> {
        for event_id in event_ids(event) {
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
            if referenced_event.content == event.content && referenced_event.pubkey != event.pubkey
            {
                debug!(
                    "Event {} is a copy of event {}",
                    event.id, referenced_event.id
                );
                return Ok(true);
            }
        }

        Ok(false)
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