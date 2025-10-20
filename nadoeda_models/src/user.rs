pub type UserId = i64;

pub struct User {
    pub id: UserId,
    pub timezone: chrono_tz::Tz,
    pub tg_chat_id: Option<i64>,
}
