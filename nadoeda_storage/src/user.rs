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

    async fn get(&self, id: UserId) -> Result<Option<User>, Self::Error>;
    async fn get_by_tg_chat(&self, chat_id: i64) -> Result<Option<User>, Self::Error>;
    async fn create(&self, new_user: NewUser) -> Result<UserId, Self::Error>;
    async fn update(&self, update_user: UpdateUser) -> Result<UserId, Self::Error>;
    async fn delete(&self, id: UserId) -> Result<(), Self::Error>;
}

mod sqlite_user_storage {
    use async_trait::async_trait;
    use nadoeda_models::{chrono_tz::{self, Tz}, user::{User, UserId}};

    use super::{NewUser, UpdateUser, UserInfoStorage};

    #[derive(sqlx::FromRow, sqlx::Type)]
    pub struct UserStorageModel {
        pub id: i64,
        pub timezone: String,
        pub tg_chat_id: Option<i64>,
    }

    impl From<User> for UserStorageModel {
        fn from(value: User) -> Self {
            Self {
                id: value.id,
                timezone: value.timezone.to_string(),
                tg_chat_id: value.tg_chat_id,
            }
        }
    }

    impl From<UserStorageModel> for User {
        fn from(value: UserStorageModel) -> Self {
            Self {
                id: value.id,
                tg_chat_id: value.tg_chat_id,
                timezone: value.timezone.parse().unwrap_or_default()
            }
        }
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

        async fn get(&self, id: UserId) -> Result<Option<User>, Self::Error> {
            let user = sqlx::query_as!(UserStorageModel, "SELECT * FROM users WHERE id = ?;", id).fetch_optional(&self.pool).await?;

            Ok(user.map(Into::into))
        }
        async fn get_by_tg_chat(&self, chat_id: i64) -> Result<Option<User>, Self::Error> {
            let user = sqlx::query_as!(UserStorageModel,
                "SELECT * FROM users WHERE tg_chat_id = ?",
                chat_id
            ).fetch_optional(&self.pool).await?;

            Ok(user.map(Into::into))
        }
        async fn create(&self, new_user: NewUser) -> Result<UserId, Self::Error> {
            let NewUser {tg_chat_id, timezone} = new_user;
            let timezone = timezone.to_string();
            let user_id = sqlx::query_scalar!(
                "INSERT INTO users (tg_chat_id, timezone)
                 VALUES (?, ?)
                 RETURNING id",
                tg_chat_id,
                timezone
            ).fetch_one(&self.pool).await?;

            Ok(user_id)
        }
        async fn update(&self, update_user: UpdateUser) -> Result<UserId, Self::Error> {
            todo!()
        }
        async fn delete(&self, id: UserId) -> Result<(), Self::Error> {
            todo!()
        }
    }
}
