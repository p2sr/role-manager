use std::collections::HashMap;
use serde::Deserialize;
use crate::boards::srcom::{Asset, Link, TimingMethod};

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum GameOrId {
    Id(GameId),
    Game {
        data: Game
    }
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct GameId(pub String);

#[derive(Deserialize, Debug, Clone)]
pub struct Game {
    pub id: GameId,
    pub names: Names,
    #[serde(rename = "boostReceived")]
    pub boosts_received: u64,
    #[serde(rename = "boostDistinctDonors")]
    pub boost_distinct_donors: u64,
    pub abbreviation: String,
    pub weblink: String,
    pub discord: Option<String>,
    pub released: u16,
    #[serde(rename = "release-date")]
    pub release_date: String,
    pub ruleset: Ruleset,
    pub romhack: bool,
    pub gametypes: Vec<String>,
    pub platforms: Vec<String>,
    pub regions: Vec<String>,
    pub genres: Vec<String>,
    pub engines: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub moderators: HashMap<String, ModeratorRole>,
    pub created: Option<String>,
    pub assets: Assets,
    pub links: Option<Vec<Link>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Names {
    pub international: String,
    pub japanese: Option<String>,
    pub twitch: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Ruleset {
    #[serde(rename = "show-milliseconds")]
    pub show_milliseconds: bool,
    #[serde(rename = "require-verification")]
    pub require_verification: bool,
    #[serde(rename = "require-video")]
    pub require_video: bool,
    #[serde(rename = "run-times")]
    pub run_times: Vec<TimingMethod>,
    #[serde(rename = "default-time")]
    pub default_time: TimingMethod,
    #[serde(rename = "emulators-allowed")]
    pub emulators_allowed: bool
}

#[derive(Deserialize, Debug, Clone)]
pub struct Assets {
    pub logo: GameAsset,
    #[serde(rename = "cover-tiny")]
    pub cover_tiny: GameAsset,
    #[serde(rename = "cover-small")]
    pub cover_small: GameAsset,
    #[serde(rename = "cover-medium")]
    pub cover_medium: GameAsset,
    #[serde(rename = "cover-large")]
    pub cover_large: GameAsset,
    pub icon: GameAsset,
    #[serde(rename = "trophy-1st")]
    pub trophy_1st: GameAsset,
    #[serde(rename = "trophy-2nd")]
    pub trophy_2nd: GameAsset,
    #[serde(rename = "trophy-3rd")]
    pub trophy_3rd: GameAsset,
    #[serde(rename = "trophy-4th")]
    pub trophy_4th: Option<GameAsset>,
    pub background: Option<GameAsset>,
    pub foreground: Option<GameAsset>
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameAsset {
    pub uri: Option<String>
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum ModeratorRole {
    #[serde(rename = "moderator")]
    Moderator,
    #[serde(rename = "super-moderator")]
    SuperModerator
}
