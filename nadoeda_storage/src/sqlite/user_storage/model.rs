use nadoeda_models::user::User;

#[derive(Debug, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use nadoeda_models::{chrono_tz, user::User};
    use proptest::prelude::*;

    fn arb_user() -> impl Strategy<Value = User> {
        (
            any::<i64>(),         // id
            any::<Option<i64>>(), // tg_chat_id
            prop_oneof![
                Just("UTC".to_string()),
                Just("Europe/Moscow".to_string()),
                Just("America/New_York".to_string()),
                Just("Asia/Tokyo".to_string()),
                ".*".prop_map(|s| s),
            ],
        )
            .prop_map(|(id, tg_chat_id, tz)| {
                let timezone = tz.parse().unwrap_or_default();
                User {
                    id,
                    tg_chat_id,
                    timezone,
                }
            })
    }

    // --- Property tests ---

    proptest! {
        #[test]
        fn test_user_roundtrip(user in arb_user()) {
            let storage: UserStorageModel = user.into();
            let restored: User = storage.into();

            prop_assert_eq!(user.id, restored.id);
            prop_assert_eq!(user.tg_chat_id, restored.tg_chat_id);

            prop_assert_eq!(
                user.timezone.to_string(),
                restored.timezone.to_string(),
                "Timezone mismatch after roundtrip"
            );
        }

        #[test]
        fn test_storage_roundtrip_random(id in any::<i64>(), tz in ".*", tg_chat_id in any::<Option<i64>>()) {
            let storage = UserStorageModel {
                id,
                timezone: tz.clone(),
                tg_chat_id,
            };

            let restored: User = storage.clone().into();
            let back: UserStorageModel = restored.into();

            prop_assert_eq!(storage.id, back.id);
            prop_assert_eq!(storage.tg_chat_id, back.tg_chat_id);

            // If invalid timezone, we expect fallback to default (usually "UTC")
            if tz.parse::<chrono_tz::Tz>().is_err() {
                prop_assert_eq!(back.timezone, "UTC".to_string());
            } else {
                prop_assert_eq!(tz, back.timezone);
            }
        }

        #[test]
        fn test_invalid_timezone_fallback(tz in ".*") {
            let storage = UserStorageModel {
                id: 1,
                timezone: tz.clone(),
                tg_chat_id: Some(42),
            };

            let user: User = storage.into();
            prop_assert!(
                !user.timezone.to_string().is_empty(),
                "Timezone fallback failed for '{}'",
                tz
            );
        }
    }
}
