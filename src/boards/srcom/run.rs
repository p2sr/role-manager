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

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct RunId(pub String);

#[derive(Deserialize, Debug, Clone)]
pub struct Run {
    pub id: RunId,
    pub weblink: String,
    pub game: GameId,
    pub level: Option<LevelId>,
    pub category: CategoryId,
    pub videos: Option<RunVideos>,
    pub comment: Option<String>,
    pub status: RunStatus,
    pub players: Vec<RunPlayer>,
    pub date: Option<String>,
    pub submitted: Option<String>,
    pub times: RunTimes,
    pub system: RunSystem,
    pub splits: Option<RunSplits>,
    pub values: HashMap<VariableId, VariableValueId>,
    pub links: Option<Vec<Link>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct RunVideos {
    pub text: Option<String>,
    pub links: Vec<RunVideoLink>
}

#[derive(Deserialize, Debug, Clone)]
pub struct RunVideoLink {
    pub uri: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "status")]
pub enum RunStatus {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "verified")]
    Verified {
        examiner: Option<UserId>,
        #[serde(rename = "verify-date")]
        verify_date: Option<String>
    },
    #[serde(rename = "rejected")]
    Rejected {
        examiner: Option<UserId>,
        reason: Option<String>
    }
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct RunTimes {
    pub primary: String,
    pub primary_t: f64,
    pub realtime: Option<String>,
    pub realtime_t: f64,
    pub realtime_noloads: Option<String>,
    pub realtime_noloads_t: f64,
    pub ingame: Option<String>,
    pub ingame_t: f64
}

#[derive(Deserialize, Debug, Clone)]
pub struct RunSystem {
    pub platform: Option<PlatformId>,
    pub emulated: bool,
    pub region: Option<RegionId>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "rel")]
pub enum RunSplits {
    #[serde(rename = "splits.io")]
    SplitsIo {
        uri: String
    }
}
