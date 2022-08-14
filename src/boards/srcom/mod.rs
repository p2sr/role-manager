pub mod game;
pub mod leaderboard;
pub mod category;
pub mod level;
pub mod platform;
pub mod region;
pub mod variable;
pub mod run;
pub mod user;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Asset {
    pub uri: String,
    pub width: u32,
    pub height: u32
}

#[derive(Deserialize, Debug)]
pub struct Link {
    pub rel: String,
    pub uri: String
}

#[derive(Deserialize, Debug)]
pub enum TimingMethod {
    #[serde(rename = "realtime")]
    RealTime,
    #[serde(rename = "realtime_noloads")]
    RealTimeNoLoads,
    #[serde(rename = "ingame")]
    InGame
}
