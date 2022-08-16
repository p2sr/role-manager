use std::collections::HashMap;
use serde::Deserialize;
use crate::boards::srcom::category::{CategoryId, CategoryOrId};
use crate::boards::srcom::game::{GameId, GameOrId};
use crate::boards::srcom::level::LevelId;
use crate::boards::srcom::{Link, MultipleItemRequest, TimingMethod};
use crate::boards::srcom::platform::PlatformId;
use crate::boards::srcom::region::RegionId;
use crate::boards::srcom::run::{Run, RunPlayer, RunStatus};
use crate::boards::srcom::user::{User, UserId};
use crate::boards::srcom::variable::{Variable, VariableId, VariableValueId};

#[derive(Deserialize, Debug, Clone)]
pub struct Leaderboard {
    pub weblink: String,
    pub game: GameOrId,
    pub category: CategoryOrId,
    pub level: Option<LevelId>,
    pub platform: Option<PlatformId>,
    pub region: Option<RegionId>,
    pub emulators: Option<bool>,
    #[serde(rename = "video-only")]
    pub video_only: bool,
    pub timing: TimingMethod,
    pub values: HashMap<VariableId, VariableValueId>,
    pub runs: Vec<LeaderboardPlace>,
    pub links: Option<Vec<Link>>,
    pub players: Option<MultipleItemRequest<UserOrGuest>>,
    pub variables: Option<MultipleItemRequest<Variable>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct LeaderboardPlace {
    pub place: u64,
    pub run: Run
}

impl Leaderboard {
    pub fn get_highest_run(&self, user_id: &UserId) -> Option<LeaderboardPlace> {
        let mut best_place: Option<LeaderboardPlace> = None;

        for run in &self.runs {
            let mut player_match = false;
            for p in &(run.run.players) {
                if let RunPlayer::User { id, .. } = p {
                    if id == user_id {
                        player_match = true;
                        break;
                    }
                }
            }

            if player_match && matches!(run.run.status, RunStatus::Verified {..}) {
                if match &best_place {
                    Some(best) => run.place < best.place,
                    None => true
                } {
                    best_place = Some(run.clone())
                }
            }
        }

        best_place
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "rel")]
pub enum UserOrGuest {
    #[serde(rename = "user")]
    User(User),
    #[serde(rename = "guest")]
    Guest {
        name: String,
        links: Option<Vec<Link>>
    }
}
