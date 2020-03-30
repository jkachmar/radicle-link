use crate::id::entity::Entity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ProjectInfo {
    pub description: String,
}

pub type Project = Entity<ProjectInfo>;
