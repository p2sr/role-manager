use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct PlatformId(pub String);

#[derive(Deserialize, Debug)]
pub struct Platform {
    pub id: PlatformId,
    pub name: String,
    pub released: u16,
    pub links: Option<Vec<Link>>
}
