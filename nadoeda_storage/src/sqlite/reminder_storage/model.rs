use nadoeda_models::reminder::{Reminder, ReminderFireTime, ReminderState};

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

#[cfg(test)]
mod tests {
    use super::*;
    use nadoeda_models::reminder::{Reminder, ReminderFireTime, ReminderState};
    use proptest::prelude::*;

    fn arb_reminder_state() -> impl Strategy<Value = ReminderState> {
        prop_oneof![
            Just(ReminderState::Pending),
            Just(ReminderState::Scheduled),
            (1u8..=10u8).prop_map(|a| ReminderState::Nagging { attempts_left: a }),
            (1u8..=10u8).prop_map(|a| ReminderState::Confirming { attempts_left: a }),
        ]
    }

    fn arb_fire_time() -> impl Strategy<Value = ReminderFireTime> {
        "12:30:00".prop_map(|_| ReminderFireTime::from_string("12:30:00").unwrap())
    }

    fn arb_reminder() -> impl Strategy<Value = Reminder> {
        (
            any::<i64>(),         // id
            any::<i64>(),         // user_id
            arb_fire_time(),      // fire_at
            ".*",                 // text
            arb_reminder_state(), // state
        )
            .prop_map(|(id, user_id, fire_at, text, state)| Reminder {
                id,
                user_id,
                fire_at,
                text,
                state,
            })
    }

    proptest! {
        #[test]
        fn test_convert_and_parse_state_roundtrip(state in arb_reminder_state()) {
            let (kind, attempts) = convert_state(state);
            let parsed = parse_state(&kind, attempts);
            match (state, parsed) {
                (ReminderState::Pending, ReminderState::Pending)
                | (ReminderState::Scheduled, ReminderState::Scheduled) => {},
                (ReminderState::Nagging { attempts_left: a1 }, ReminderState::Nagging { attempts_left: a2 })
                | (ReminderState::Confirming { attempts_left: a1 }, ReminderState::Confirming { attempts_left: a2 }) => {
                    prop_assert_eq!(a1 as i64, a2 as i64);
                },
                (s1, s2) => prop_assert_eq!(s1, s2, "State mismatch after roundtrip")
            }
        }

        #[test]
        fn test_reminder_roundtrip(reminder in arb_reminder()) {
            let storage: ReminderStorageModel = reminder.clone().into();
            let restored: Reminder = storage.into();

            prop_assert_eq!(reminder.id, restored.id);
            prop_assert_eq!(reminder.user_id, restored.user_id);
            prop_assert_eq!(reminder.text, restored.text);
            prop_assert_eq!(reminder.fire_at.into_string(), restored.fire_at.into_string());

            let (kind, attempts) = convert_state(reminder.state);
            let (kind2, attempts2) = convert_state(restored.state);

            prop_assert_eq!(kind, kind2, "State kind mismatch after roundtrip");
            prop_assert_eq!(attempts, attempts2, "Attempts mismatch after roundtrip");
        }

        #[test]
        fn test_parse_state_handles_unknown_strings(s in ".*") {
            // Any non-matching state string should default to Pending
            let parsed = parse_state(&s, Some(5));
            if s != "Pending" && s != "Scheduled" && s != "Nagging" && s != "Confirming" {
                match parsed {
                    ReminderState::Pending => {},
                    _ => prop_assert!(false, "Unexpected state for unknown string: {}", s),
                }
            }
        }

        #[test]
        fn test_convert_state_consistency(state in arb_reminder_state()) {
            let (kind, attempts) = convert_state(state);
            match (kind.as_str(), attempts, state) {
                ("Pending", None, ReminderState::Pending) => {},
                ("Scheduled", None, ReminderState::Scheduled) => {},
                ("Nagging", Some(a), ReminderState::Nagging { attempts_left }) => prop_assert_eq!(a, attempts_left as i64),
                ("Confirming", Some(a), ReminderState::Confirming { attempts_left }) => prop_assert_eq!(a, attempts_left as i64),
                (k, _, s) => prop_assert!(false, "Invalid conversion: kind={}, state={:?}", k, s),
            }
        }
    }
}
