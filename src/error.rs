use std::fmt::{Display, Formatter};
use sea_orm::DbErr;
use serenity::prelude::SerenityError;

#[derive(Debug)]
pub struct RoleManagerError {
    pub cause: String,
    pub report_via_edit: bool
}

impl RoleManagerError {
    pub fn new(cause: String) -> Self {
        RoleManagerError {
            cause,
            report_via_edit: false
        }
    }

    pub fn new_edit(cause: String) -> Self {
        RoleManagerError {
            cause,
            report_via_edit: true
        }
    }
}

impl std::error::Error for RoleManagerError {}

impl Display for RoleManagerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cause: {}", self.cause)
    }
}

impl From<String> for RoleManagerError {
    fn from(cause: String) -> Self {
        Self { cause, report_via_edit: false }
    }
}

impl From<&str> for RoleManagerError {
    fn from(cause: &str) -> Self {
        Self { cause: cause.to_string(), report_via_edit: false }
    }
}

impl From<SerenityError> for RoleManagerError {
    fn from(err: SerenityError) -> Self {
        Self {
            cause: format!("Discord error: {}", err),
            report_via_edit: false
        }
    }
}

impl From<DbErr> for RoleManagerError {
    fn from(err: DbErr) -> Self {
        Self {
            cause: format!("Database error: {}", err),
            report_via_edit: false
        }
    }
}

impl From<std::io::Error> for RoleManagerError {
    fn from(err: std::io::Error) -> Self {
        Self {
            cause: format!("IO error: {}", err),
            report_via_edit: false
        }
    }
}

impl From<json5::Error> for RoleManagerError {
    fn from(err: json5::Error) -> Self {
        Self {
            cause: format!("Json5 Parser Error: {}", err),
            report_via_edit: false
        }
    }
}

impl From<std::fmt::Error> for RoleManagerError {
    fn from(err: std::fmt::Error) -> Self {
        Self {
            cause: format!("Formatter Error: {}", err),
            report_via_edit: false
        }
    }
}

impl From<serde_json::Error> for RoleManagerError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            cause: format!("Json Parser Error: {}", err),
            report_via_edit: false
        }
    }
}
