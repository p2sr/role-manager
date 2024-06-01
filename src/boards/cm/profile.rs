use std::sync::Arc;
use chrono::NaiveDateTime;
use serde::Deserialize;
use crate::error::RoleManagerError;

#[derive(Deserialize, Clone, Debug)]
pub struct Profile {
    #[serde(rename = "boardname")]
    pub board_name: Option<String>,
    pub avatar: Option<String>
}

#[derive(Deserialize, Debug)]
pub struct ProfileResponse {
    #[serde(rename = "profileNumber")]
    pub profile_number: String,
    #[serde(rename = "userData")]
    pub user_data: Profile
}

#[derive(Debug)]
pub struct CachedProfile {
    pub profile: Arc<Profile>,
    pub fetched_at: NaiveDateTime
}

pub async fn fetch_profile(id: i64) -> Result<Profile, RoleManagerError> {
    Ok(reqwest::get(format!("https://board.portal2.sr/profile/{}/json", id))
        .await.map_err(|err| format!("Failed to request profile for steam id {} on board.portal2.sr: {}", id, err))?
        .json::<ProfileResponse>()
        .await.map_err(|err| format!("Failed to convert response from profile for steam id {} on board.portal2.sr: {}", id, err))?
        .user_data)
}
