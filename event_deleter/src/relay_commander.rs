use crate::event_analyzer::DeleteRequest;
use async_trait::async_trait;
use nostr_sdk::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use tokio::process::Command;

#[derive(Clone)]
pub struct RelayCommander<T: RawCommanderTrait> {
    raw_commander: T,
}

impl<T: RawCommanderTrait> RelayCommander<T> {
    pub fn new(raw_commander: T) -> Self {
        RelayCommander { raw_commander }
    }
}

impl Default for RelayCommander<RawCommander> {
    fn default() -> Self {
        let raw_commander = RawCommander {};
        RelayCommander::new(raw_commander)
    }
}

impl<T: RawCommanderTrait> RelayCommander<T> {
    pub async fn execute_delete(
        &self,
        delete_reason: Vec<DeleteRequest>,
        dry_run: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut ids = HashSet::new();
        let mut authors = HashSet::new();

        for reason in delete_reason {
            match reason {
                DeleteRequest::ReplyCopy(id) => {
                    ids.insert(id);
                }
                DeleteRequest::ForbiddenName(pubkey) => {
                    authors.insert(pubkey);
                }
                DeleteRequest::Vanish(_, pubkey, _) => {
                    authors.insert(pubkey);
                }
            }
        }

        if !ids.is_empty() {
            let ids_filter = Filter::new().ids(ids);
            self.raw_commander
                .delete_from_filter(ids_filter, dry_run)
                .await?;
        }

        if !authors.is_empty() {
            let authors_filter = Filter::new().authors(authors);
            self.raw_commander
                .delete_from_filter(authors_filter, dry_run)
                .await?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct RawCommander {}

#[async_trait]
impl RawCommanderTrait for RawCommander {}
#[async_trait]
pub trait RawCommanderTrait: Sync + Send + 'static {
    async fn delete_from_filter(
        &self,
        filter: Filter,
        dry_run: bool,
    ) -> std::result::Result<(), Box<dyn Error>> {
        let json_filter = filter.as_json();
        let command_str = format!(
            "./strfry delete --filter='{}' {}",
            json_filter,
            if dry_run { "--dry-run" } else { "" }
        );

        let status = Command::new("bash")
            .arg("-c")
            .arg(&command_str)
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Delete command failed with status: {}", status).into())
        }
    }
}
