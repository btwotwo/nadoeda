use async_trait::async_trait;
use nadoeda_models::{
    chrono, chrono_tz,
    user::{User, UserId},
};

pub struct NewUser {
    timezone: chrono_tz::Tz,
    tg_chat_id: Option<i64>,
}

pub struct UpdateUser {
    timezone: Option<chrono_tz::Tz>,
    tg_chat_id: Option<i64>,
}

#[async_trait]
pub trait UserInfoStorage {
    type Error: Send + Sync + 'static;

    async fn get(id: UserId) -> Result<Option<User>, Self::Error>;
    async fn get_by_tg_chat(chat_id: i64) -> Result<Option<User>, Self::Error>;
    async fn create(new_user: NewUser) -> Result<UserId, Self::Error>;
    async fn update(update_user: UpdateUser) -> Result<UserId, Self::Error>;
    async fn delete(id: UserId) -> Result<(), Self::Error>;
}

pub struct SqliteUserInfoStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteUserInfoStorage {
    pub fn new(pool: sqlx::SqlitePool) -> anyhow::Result<Self> {
        Ok(Self { pool })
    }
}

#[async_trait]
impl UserInfoStorage for SqliteUserInfoStorage {
    type Error = anyhow::Error;
    
    async fn get(id: UserId) -> Result<Option<User>, Self::Error> {
        let user = sqlx::query_as!(User, "SELECT * FROM users;");

        todo!()
    }
    async fn get_by_tg_chat(chat_id: i64) -> Result<Option<User>, Self::Error> {
        todo!()
    }
    async fn create(new_user: NewUser) -> Result<UserId, Self::Error> {
        todo!()
    }
    async fn update(update_user: UpdateUser) -> Result<UserId, Self::Error> {
        todo!()
    }
    async fn delete(id: UserId) -> Result<(), Self::Error> {
        todo!()
    }
}
