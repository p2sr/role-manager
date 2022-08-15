use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct RegionId(pub String);

#[derive(Deserialize, Debug)]
pub struct Region {
    pub id: RegionId,
    pub name: String,
    pub links: Vec<Link>
}
