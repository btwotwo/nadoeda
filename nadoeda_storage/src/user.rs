use async_trait::async_trait;
use nadoeda_models::{chrono, chrono_tz, user::{User, UserId}};


pub struct NewUser {
    timezone: chrono_tz::Tz,
    tg_chat_id: Option<i64>,
}

pub struct UpdateUser {
    timezone: Option<chrono_tz::Tz>,
    tg_chat_id: Option<i64>
}

#[async_trait]
pub trait UserInfoStorage {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn get(id: UserId) -> Result<Option<User>, Self::Error>;
    async fn create(new_user: NewUser) -> Result<UserId, Self::Error>;
    async fn update(update_user: UpdateUser) -> Result<UserId, Self::Error>;
    async fn delete(id: UserId) -> Result<(), Self::Error>;
}

