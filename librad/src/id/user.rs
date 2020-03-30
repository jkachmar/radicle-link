use crate::id::entity::Entity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub email: String,
}

pub type User = Entity<UserInfo>;
