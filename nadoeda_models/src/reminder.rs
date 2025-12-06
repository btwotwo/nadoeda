use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Utc};

use crate::user::UserId;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

    pub fn with_timezone(self, timezone: impl chrono::TimeZone) -> Result<Self, &'static str> {
        let today_local = timezone
            .from_utc_datetime(&Utc::now().naive_utc())
            .date_naive();

        let date =
            NaiveDate::from_ymd_opt(today_local.year(), today_local.month(), today_local.day())
                .expect("The date is always valid.");

        let naive_dt = date.and_time(self.0);
        let local_dt = timezone
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or("Daylight savings time encountered!")?;

        let utc_dt = local_dt.with_timezone(&Utc);

        Ok(Self(utc_dt.time()))
    }

    pub fn time(&self) -> &chrono::NaiveTime {
        &self.0
    }

    pub fn into_time(self) -> chrono::NaiveTime {
        self.0
    }

    pub fn into_string(self) -> String {
        self.0.format("%H:%M:%S").to_string()
    }

    pub fn from_string(input: &str) -> Option<Self> {
        let naive_time = NaiveTime::parse_from_str(input, "%H:%M:%S");
        naive_time.ok().map(Self::new)
    }
}

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: ReminderId,
    pub state: ReminderState,
    pub fire_at: ReminderFireTime,
    pub text: String,
    pub user_id: UserId,
}
