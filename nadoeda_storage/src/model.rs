use nadoeda_models::reminder::{ReminderFireTime, ReminderId};

pub struct NewReminder {
    pub text: String,
    pub fire_at: ReminderFireTime,
}

pub struct UpdateReminder {
    pub id: ReminderId,
    pub text: Option<String>,
    pub fire_at: Option<ReminderFireTime>,
}
