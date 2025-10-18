pub type UserId = u64;

pub struct User {
    pub id: UserId,
    pub timezone: chrono_tz::Tz,
    pub tg_chat_id: i64,
}
