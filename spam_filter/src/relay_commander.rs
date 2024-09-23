use nostr_sdk::prelude::*;
use std::error::Error;
use tokio::process::Command;

#[derive(Clone, Default)]
pub struct RelayCommander;

impl RelayCommander {
    pub async fn execute_delete(
        &self,
        ids: Vec<EventId>,
        dry_run: bool,
    ) -> Result<(), Box<dyn Error>> {
        let filter = Filter::new().ids(ids.to_vec());
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
