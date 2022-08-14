use serde::Deserialize;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug)]
pub struct UserId(pub String);

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: UserId,
    pub names: Names,
    pub pronouns: Option<String>,
    pub weblink: String,
    pub role: UserRole,
    pub signup: String,
    pub location: Option<UserLocation>,
    pub twitch: Option<UserConnection>,
    pub hitbox: Option<UserConnection>,
    pub youtube: Option<UserConnection>,
    pub twitter: Option<UserConnection>,
    pub speedrunslive: Option<UserConnection>,
    pub links: Vec<Link>
}

#[derive(Deserialize, Debug)]
pub struct Names {
    pub international: String,
    pub japanese: Option<String>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "style")]
pub enum NameStyle {
    #[serde(rename = "gradient")]
    Gradient {
        color_from: NameStyleColor,
        color_to: NameStyleColor
    },
    #[serde(rename = "solid")]
    Solid {
        color: NameStyleColor
    }
}

#[derive(Deserialize, Debug)]
pub struct NameStyleColor {
    pub light: String,
    pub dark: String
}

#[derive(Deserialize, Debug)]
pub enum UserRole {
    #[serde(rename = "banned")]
    Banned,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "trusted")]
    Trusted,
    #[serde(rename = "moderator")]
    Moderator,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "programmer")]
    Programmer
}

#[derive(Deserialize, Debug)]
pub struct UserLocation {
    country: UserLocationSpec,
    region: Option<UserLocationSpec>
}

#[derive(Deserialize, Debug)]
pub struct UserLocationSpec {
    code: String,
    names: Names
}

#[derive(Deserialize, Debug)]
pub struct UserConnection {
    uri: String
}
