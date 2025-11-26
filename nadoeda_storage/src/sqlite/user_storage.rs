mod model;

use async_trait::async_trait;
use model::UserStorageModel;
use nadoeda_models::user::{User, UserId};

use crate::user::{NewUser, UserInfoStorage};

pub struct SqliteUserInfoStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteUserInfoStorage {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserInfoStorage for SqliteUserInfoStorage {
    type Error = anyhow::Error;

    async fn get(&self, id: UserId) -> Result<Option<User>, Self::Error> {
        let user = sqlx::query_as!(UserStorageModel, "SELECT * FROM users WHERE id = ?;", id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user.map(Into::into))
    }
    async fn get_by_tg_chat(&self, chat_id: i64) -> Result<Option<User>, Self::Error> {
        let user = sqlx::query_as!(
            UserStorageModel,
            "SELECT * FROM users WHERE tg_chat_id = ?",
            chat_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user.map(Into::into))
    }
    async fn create(&self, new_user: NewUser) -> Result<User, Self::Error> {
        let NewUser {
            tg_chat_id,
            timezone,
        } = new_user;
        let timezone = timezone.to_string();
        let user = sqlx::query_as!(
            UserStorageModel,
            "INSERT INTO users (tg_chat_id, timezone)
                 VALUES (?, ?)
                 RETURNING *",
            tg_chat_id,
            timezone
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user.into())
    }
    async fn update(&self, update_user: User) -> Result<User, Self::Error> {
        let User {
            id,
            tg_chat_id,
            timezone,
        } = update_user;
        let timezone = timezone.to_string();
        let user = sqlx::query_as!(
            UserStorageModel,
            "UPDATE users
                 SET tg_chat_id = ?,
                     timezone = ?
                 WHERE id = ?
                 RETURNING *",
            tg_chat_id,
            timezone,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user.into())
    }
    async fn delete(&self, id: UserId) -> Result<(), Self::Error> {
        sqlx::query_as!(UserStorageModel, "DELETE FROM users WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
