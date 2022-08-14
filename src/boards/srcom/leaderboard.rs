use std::collections::HashMap;
use serde::Deserialize;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::level::LevelId;
use crate::boards::srcom::{Link, TimingMethod};
use crate::boards::srcom::platform::PlatformId;
use crate::boards::srcom::region::RegionId;
use crate::boards::srcom::run::Run;
use crate::boards::srcom::variable::{VariableId, VariableValueId};

#[derive(Deserialize, Debug)]
pub struct Leaderboard {
    weblink: String,
    game: GameId,
    category: CategoryId,
    level: Option<LevelId>,
    platform: Option<PlatformId>,
    region: Option<RegionId>,
    emulators: Option<bool>,
    #[serde(rename = "video-only")]
    video_only: bool,
    timing: TimingMethod,
    values: HashMap<VariableId, VariableValueId>,
    runs: Vec<LeaderboardPlace>,
    links: Vec<Link>
}

#[derive(Deserialize, Debug)]
pub struct LeaderboardPlace {
    pub place: u64,
    pub run: Run
}
