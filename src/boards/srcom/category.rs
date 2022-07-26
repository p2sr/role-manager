use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CategoryOrId {
    Id(CategoryId),
    Category { data: Category }
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct CategoryId(pub String);

#[derive(Deserialize, Debug, Clone)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub weblink: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub rules: String,
    pub players: PlayerCount,
    pub miscellaneous: bool,
    pub links: Option<Vec<Link>>
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(tag = "type", content = "value")]
pub enum PlayerCount {
    #[serde(rename = "exactly")]
    Exactly(u64),
    #[serde(rename = "up-to")]
    UpTo(u64)
}
