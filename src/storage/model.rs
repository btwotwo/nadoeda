use crate::reminder::{ReminderFireTime, ReminderId, ReminderState};

pub struct NewReminder {
    pub text: String,
    pub fire_at: ReminderFireTime,
}

pub struct UpdateReminder {
    pub id: ReminderId,
    pub text: Option<String>,
    pub fire_at: Option<ReminderFireTime>,
}
