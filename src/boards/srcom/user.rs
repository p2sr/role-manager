use std::fmt::{Display, Formatter};
use serde::{de, Deserialize, Deserializer};
use serde::de::Visitor;
use crate::boards::srcom::Link;
use crate::error::RoleManagerError;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct UserId([u8; 8]);

impl TryFrom<&str> for UserId {
    type Error = RoleManagerError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if v.len() != 8 {
            return Err(RoleManagerError::new(format!("SRC ids must be of size 8, found {}", v.len())));
        }
        let mut user_id = [0; 8];
        user_id.copy_from_slice(v.as_bytes());
        Ok(UserId(user_id))
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for i in self.0.iter() {
            write!(f, "{}", *i as char)?
        }

        Ok(())
    }
}

impl <'de> Deserialize<'de> for UserId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct IdVisitor;

        impl <'de> Visitor<'de> for IdVisitor {
            type Value = UserId;

            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                write!(f, "Speedrun.com ID must be an 8-character string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
                Ok(UserId::try_from(v).map_err(|e| de::Error::custom(e))?)
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}


#[derive(Deserialize, Debug, Clone)]
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
    pub links: Option<Vec<Link>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Names {
    pub international: String,
    pub japanese: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct NameStyleColor {
    pub light: String,
    pub dark: String
}

#[derive(Deserialize, Debug, Copy, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct UserLocation {
    country: UserLocationSpec,
    region: Option<UserLocationSpec>
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct UserLocationSpec {
    code: String,
    names: Names
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct UserConnection {
    uri: String
}
