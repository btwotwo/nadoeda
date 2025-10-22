use nadoeda_models::user::User;

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
            timezone: value.timezone.parse().unwrap_or_default(),
        }
    }
}
