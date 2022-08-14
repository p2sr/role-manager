use std::fmt::{Display, Formatter};
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

    pub fn newEdit(cause: String) -> Self {
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
