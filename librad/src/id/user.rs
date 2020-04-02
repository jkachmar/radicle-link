use crate::id::entity::{data::EntityData, Entity, Error};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub email: String,
}

pub type UserData = EntityData<UserInfo>;

impl UserData {
    pub fn set_email(mut self, email: String) -> Self {
        self.info.email = email;
        self
    }

    pub fn build(self) -> Result<User, Error> {
        User::from_data(self)
    }
}

pub type User = Entity<UserInfo>;
