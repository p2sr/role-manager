use std::collections::HashMap;
use serde::Deserialize;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::level::LevelId;
use crate::boards::srcom::Link;
use crate::boards::srcom::platform::PlatformId;
use crate::boards::srcom::region::RegionId;
use crate::boards::srcom::user::UserId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};

#[derive(Deserialize, Debug)]
pub struct RunId(pub String);

#[derive(Deserialize, Debug)]
pub struct Run {
    pub id: RunId,
    pub weblink: String,
    pub game: GameId,
    pub level: LevelId,
    pub category: CategoryId,
    pub videos: RunVideos,
    pub comment: String,
    pub status: RunStatus,
    pub players: Vec<RunPlayer>,
    pub date: Option<String>,
    pub submitted: Option<String>,
    pub times: RunTimes,
    pub system: RunSystem,
    pub splits: Option<RunSplits>,
    pub values: HashMap<VariableId, VariableValueId>,
    pub links: Vec<Link>
}

#[derive(Deserialize, Debug)]
pub struct RunVideos {
    pub text: String,
    pub links: Vec<RunVideoLink>
}

#[derive(Deserialize, Debug)]
pub struct RunVideoLink {
    pub uri: String
}

#[derive(Deserialize, Debug)]
#[serde(tag = "status")]
pub enum RunStatus {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "verified")]
    Verified {
        examiner: UserId,
        #[serde(rename = "verify-date")]
        verify_date: Option<String>
    },
    #[serde(rename = "rejected")]
    Rejected {
        examiner: UserId,
        reason: String
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "rel")]
pub enum RunPlayer {
    #[serde(rename = "user")]
    User {
        id: UserId,
        uri: String
    },
    #[serde(rename = "guest")]
    Guest {
        name: String,
        uri: String
    }
}

#[derive(Deserialize, Debug)]
pub struct RunTimes {
    pub primary: String,
    pub primary_t: u64,
    pub realtime: Option<String>,
    pub realtime_t: u64,
    pub realtime_noloads: Option<String>,
    pub realtime_noloads_t: u64,
    pub ingame: Option<String>,
    pub ingame_t: u64
}

#[derive(Deserialize, Debug)]
pub struct RunSystem {
    pub platform: PlatformId,
    pub emulated: bool,
    pub region: Option<RegionId>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "rel")]
pub enum RunSplits {
    #[serde(rename = "splits.io")]
    SplitsIo {
        uri: String
    }
}
