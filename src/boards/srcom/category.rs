use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug)]
pub struct CategoryId(pub String);

#[derive(Deserialize, Debug)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub weblink: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub rules: String,
    pub players: PlayerCount,
    pub miscellaneous: bool,
    pub links: Vec<Link>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
pub enum PlayerCount {
    #[serde(rename = "exactly")]
    Exactly(u64),
    #[serde(rename = "up-to")]
    UpTo(u64)
}
