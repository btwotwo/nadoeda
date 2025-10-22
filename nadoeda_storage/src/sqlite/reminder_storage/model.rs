use nadoeda_models::{
    chrono::NaiveTime,
    reminder::{Reminder, ReminderFireTime, ReminderState},
    user::UserId,
};

pub struct ReminderStorageModel {
    pub id: i64,
    pub user_id: i64,
    pub state_kind: String,
    pub attempts_left: Option<i64>,
    pub fire_at: String,
    pub text: String,
}

impl From<Reminder> for ReminderStorageModel {
    fn from(value: Reminder) -> Self {
        let (state, attempts_left) = convert_state(value.state);
        Self {
            id: value.id,
            user_id: value.user_id,
            text: value.text,
            fire_at: value.fire_at.into_string(),
            state_kind: state,
            attempts_left,
        }
    }
}

impl From<ReminderStorageModel> for Reminder {
    fn from(value: ReminderStorageModel) -> Self {
        let state = parse_state(&value.state_kind, value.attempts_left);
        let fire_at = ReminderFireTime::from_string(&value.fire_at).unwrap();
        Self {
            id: value.id,
            user_id: value.user_id,
            fire_at,
            text: value.text,
            state,
        }
    }
}

pub fn convert_state(state: ReminderState) -> (String, Option<i64>) {
    match state {
        ReminderState::Pending => ("Pending".to_string(), None),
        ReminderState::Scheduled => ("Scheduled".to_string(), None),
        ReminderState::Nagging { attempts_left } => {
            ("Nagging".to_string(), Some(attempts_left as i64))
        }
        ReminderState::Confirming { attempts_left } => {
            ("Confirming".to_string(), Some(attempts_left as i64))
        }
    }
}

pub fn parse_state(state: &str, attempts_left: Option<i64>) -> ReminderState {
    match state {
        "Pending" => ReminderState::Pending,
        "Scheduled" => ReminderState::Scheduled,
        "Nagging" => ReminderState::Nagging {
            attempts_left: attempts_left.unwrap_or(3) as u8,
        },
        "Confirming" => ReminderState::Confirming {
            attempts_left: attempts_left.unwrap_or(3) as u8,
        },
        other => {
            log::warn!("Warning: Unknown state {}, defaulting to Pending", other);
            ReminderState::Pending
        }
    }
}
