use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct RoleManagerError {
    pub cause: String
}

impl std::error::Error for RoleManagerError {}

impl Display for RoleManagerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cause: {}", self.cause)
    }
}
