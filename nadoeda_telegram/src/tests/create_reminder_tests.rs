use nadoeda_models::{chrono_tz, user::User};
use nadoeda_scheduler::ReminderScheduler;
use nadoeda_storage::sqlite;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    dptree::deps,
};
use teloxide_tests::{MockBot, MockMessageText};

use crate::{create_daily_reminder::schema, *};

use crate::tests::test_utils::*;

#[sqlite::sqlx::test]
async fn test(pool: Pool<Sqlite>) {
    let reminder_storage = storage(pool.clone());
    let user_storage = storage(pool.clone());

    let scheduler: Arc<dyn ReminderScheduler> = Arc::new(NoopReminderScheduler);
    let schema = dialogue::enter::<
        Update,
        InMemStorage<AuthenticatedActionState>,
        AuthenticatedActionState,
        _,
    >()
    .branch(schema());
    let mut bot = MockBot::new(MockMessageText::new().text("New Reminder"), schema);

    bot.dependencies(deps![
        reminder_storage,
        user_storage,
        scheduler,
        InMemStorage::<AuthenticatedActionState>::new(),
        AuthenticationInfo(User {
            id: 0,
            tg_chat_id: None,
            timezone: chrono_tz::Tz::Europe__Prague,
        }),
        AuthenticatedActionState::CreatingDailyReminder(
            CreatingDailyReminderState::WaitingForReminderText
        )
    ]);

    bot.set_state(AuthenticatedActionState::CreatingDailyReminder(
        CreatingDailyReminderState::WaitingForReminderText,
    ))
    .await;

    bot.dispatch_and_check_state(AuthenticatedActionState::CreatingDailyReminder(
        CreatingDailyReminderState::WaitingForFiringTime {
            text: "New Reminder".to_string(),
        },
    ))
    .await;
}
