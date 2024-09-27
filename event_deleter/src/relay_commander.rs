use crate::event_analyzer::RejectReason;
use nostr_sdk::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use tokio::process::Command;

#[derive(Clone, Default)]
pub struct RelayCommander;

impl RelayCommander {
    pub async fn execute_delete(
        &self,
        reject_reasons: Vec<RejectReason>,
        dry_run: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut ids = HashSet::new();
        let mut authors = HashSet::new();

        for reason in reject_reasons {
            match reason {
                RejectReason::ReplyCopy(id) => {
                    ids.insert(id);
                }
                RejectReason::ForbiddenName(pubkey) => {
                    authors.insert(pubkey);
                }
            }
        }

        if !ids.is_empty() {
            let ids_filter = Filter::new().ids(ids);
            delete_from_filter(ids_filter, dry_run).await?;
        }

        if !authors.is_empty() {
            let authors_filter = Filter::new().authors(authors);
            delete_from_filter(authors_filter, dry_run).await?;
        }

        Ok(())
    }
}

async fn delete_from_filter(
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
