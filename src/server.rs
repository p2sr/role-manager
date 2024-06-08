use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::analyzer::role_definition::{BadgeDefinition, RoleDefinition};
use crate::error::RoleManagerError;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ServerConfig {
    pub badge_roles: HashMap<String, u64>
}

impl ServerConfig {
    pub async fn read(server_id: u64) -> Result<Option<ServerConfig>, RoleManagerError> {
        tokio::fs::create_dir_all("server_configs").await?;
        let config_path = format!("server_configs/{}.json5", server_id);

        if tokio::fs::try_exists(&config_path).await? {
            let config_content = tokio::fs::read_to_string(&config_path).await?;
            Ok(Some(serde_json::from_str(&config_content)?))
        } else {
            Ok(None)
        }
    }

    pub async fn write(&self, server_id: u64) -> Result<(), RoleManagerError> {
        tokio::fs::create_dir_all("server_configs").await?;
        let config_path = format!("server_configs/{}.json5", server_id);

        tokio::fs::write(config_path, serde_json::to_string(&self)?).await?;

        Ok(())
    }

    pub fn valid_badges<'a>(&self, definition: &'a RoleDefinition) -> HashMap<&'a BadgeDefinition, u64> {
        HashMap::from_iter(definition.badges.iter()
            .filter_map(move |badge_definition| {
                match self.badge_roles.get(&badge_definition.name) {
                    Some(role_id) => Some((badge_definition, *role_id)),
                    None => None
                }
            }))
    }
}
