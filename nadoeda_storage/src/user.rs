use async_trait::async_trait;
use nadoeda_models::{
    chrono, chrono_tz,
    user::{User, UserId},
};

pub struct NewUser {
    pub timezone: chrono_tz::Tz,
    pub tg_chat_id: Option<i64>,
}

#[async_trait]
pub trait UserInfoStorage {
    type Error: Send + Sync + 'static;

    async fn get(&self, id: UserId) -> Result<Option<User>, Self::Error>;
    async fn get_by_tg_chat(&self, chat_id: i64) -> Result<Option<User>, Self::Error>;
    async fn create(&self, new_user: NewUser) -> Result<User, Self::Error>;
    async fn update(&self, update_user: User) -> Result<User, Self::Error>;
    async fn delete(&self, id: UserId) -> Result<(), Self::Error>;
}
