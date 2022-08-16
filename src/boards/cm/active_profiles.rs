use chrono::NaiveDateTime;
use serde::Deserialize;
use crate::error::RoleManagerError;

#[derive(Debug)]
pub struct CachedActiveProfiles {
    pub active_profiles: Vec<String>,
    pub fetched_at: NaiveDateTime
}

#[derive(Deserialize, Debug)]
struct ActiveProfilesResponse {
    profiles: Vec<String>
}

pub async fn fetch_active_profiles(months: u64) -> Result<Vec<String>, RoleManagerError> {
    let client = reqwest::Client::new();
    Ok(client.post("https://board.portal2.sr/api-v2/active-profiles")
        .form(&[("months", months)])
        .send()
        .await.map_err(|err| format!("Failed to request active profiles on board.portal2.sr: {}", err))?
        .json::<ActiveProfilesResponse>()
        .await.map_err(|err| format!("Failed to convert response from active profiles on board.portal2.sr: {}", err))?
        .profiles)
}
