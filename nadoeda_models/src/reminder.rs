use chrono::Timelike;

use crate::user::UserId;

#[derive(Debug, Copy, Clone)]
pub enum ReminderState {
    Pending,
    Scheduled,
    Nagging { attempts_left: u8 },
    Confirming { attempts_left: u8 },
}

pub enum ReminderFiringPeriod {
    OneOff,
    Daily,
}

pub type ReminderId = i64;

#[derive(Debug, Clone)]
pub struct ReminderFireTime(chrono::NaiveTime);

impl ReminderFireTime {
    pub fn new(inner: chrono::NaiveTime) -> Self {
        let normalized_time = inner.with_nanosecond(0).expect("Will never fail.");
        Self(normalized_time)
    }

    pub fn time(&self) -> &chrono::NaiveTime {
        &self.0
    }

    pub fn into_time(self) -> chrono::NaiveTime {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: ReminderId,
    pub state: ReminderState,
    pub fire_at: ReminderFireTime,
    pub text: String,
    pub user_id: UserId
}
