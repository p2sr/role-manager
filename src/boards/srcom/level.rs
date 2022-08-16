use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct LevelId(pub String);

pub struct Level {
    pub id: LevelId,
    pub name: String,
    pub weblink: String,
    pub rules: String,
    pub links: Vec<Link>
}