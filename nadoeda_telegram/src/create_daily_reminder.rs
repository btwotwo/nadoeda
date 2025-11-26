use std::error::Error;
use std::sync::Arc;

use chrono::NaiveTime;
use dptree::case;
use nadoeda_scheduler::{ReminderScheduler, ScheduleRequest};
use nadoeda_storage::sqlite::reminder_storage::SqliteReminderStorage;
use nadoeda_storage::{NewReminder, ReminderStorage};
use teloxide::dispatching::UpdateHandler;
use teloxide::dptree::filter_map;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{Bot, types::Message};

use nadoeda_models::reminder::{Reminder, ReminderFireTime, ReminderState};

use crate::AuthenticationInfo;
use crate::util::AuthInfoExtractor;

use super::{GlobalCommand, GlobalDialogue, GlobalState, HandlerResult};

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum CreatingDailyReminderState {
    #[default]
    Start,
    WaitingForReminderText,
    WaitingForFiringTime {
        text: String,
    },
    WaitingForConfirmation {
        text: String,
        firing_time: NaiveTime,
    },
}

async fn create_daily_reminder_start(
    bot: Bot,
    dialogue: GlobalDialogue,
    auth: AuthenticationInfo,
    msg: Message,
) -> HandlerResult {
    bot.send_message(
            msg.chat.id,
            "Creating a new daily reminder! Please input reminder text. If you want to cancel, use the /cancel command.",
        )
        .await?;

    dialogue
        .update(GlobalState::CreatingDailyReminder(
            auth,
            CreatingDailyReminderState::WaitingForReminderText,
        ))
        .await?;

    Ok(())
}
async fn receive_reminder_text(
    bot: Bot,
    dialogue: GlobalDialogue,
    auth: AuthenticationInfo,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let message = format!(
                "Great! You will be reminded about \"{}\"\nNow, please enter time when reminder is going to be fired (e.g. 13:00)",
                teloxide::utils::markdown::escape(text)
            );
            bot.send_message(msg.chat.id, message).await?;
            dialogue
                .update(GlobalState::CreatingDailyReminder(
                    AuthenticationInfo(0),
                    CreatingDailyReminderState::WaitingForFiringTime {
                        text: text.to_string(),
                    },
                ))
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Please send me reminder text.")
                .await?;
        }
    }

    Ok(())
}
async fn receive_firing_time(
    bot: Bot,
    dialogue: GlobalDialogue,
    text: String,
    auth: AuthenticationInfo,
    msg: Message,
) -> HandlerResult {
    match msg
        .text()
        .map(|text| NaiveTime::parse_from_str(text, "%H:%M"))
    {
        Some(Ok(time)) => {
            let message_text = format!(
                "You will be reminded every day at *\"{}\"*
Reminder text is *\"{}\"*
If it's okay, please press *Confirm*
If you want to change something, please type /cancel and start over",
                time.format("%H:%M"),
                text
            );

            let ok_button = InlineKeyboardButton::callback("Confirm", "Confirm");
            let keyboard = InlineKeyboardMarkup::new(vec![vec![ok_button]]);

            dialogue
                .update(GlobalState::CreatingDailyReminder(
                    AuthenticationInfo(0),
                    CreatingDailyReminderState::WaitingForConfirmation {
                        text,
                        firing_time: time,
                    },
                ))
                .await?;

            bot.send_message(msg.chat.id, message_text)
                .reply_markup(keyboard)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "Could not parse time. Please send time in the following format: *13:00*",
            )
            .await?;
        }
    }
    Ok(())
}

async fn confirm_reminder(
    storage: Arc<SqliteReminderStorage>,
    bot: Bot,
    dialogue: GlobalDialogue,
    (text, firing_time): (String, NaiveTime),
    auth: AuthenticationInfo,
    query: CallbackQuery,
    scheduler: Arc<dyn ReminderScheduler>,
) -> HandlerResult {
    let reminder = NewReminder {
        text,
        fire_at: ReminderFireTime::new(firing_time),
        user_id: 0,
    };

    let reminder = storage.insert(reminder).await?;
    bot.answer_callback_query(query.id).await?;

    log::info!("Created reminder with id {}", reminder.id);

    scheduler
        .schedule_reminder(ScheduleRequest::new(reminder))
        .await?;

    bot.send_message(dialogue.chat_id(), "Reminder saved and scheduled.")
        .await?;

    dialogue.exit().await?;
    Ok(())
}

pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(teloxide::filter_command::<GlobalCommand, _>().branch(
                    case![GlobalState::Authenticated(auth)].branch(
                        case![GlobalCommand::CreateReminder].endpoint(create_daily_reminder_start),
                    ),
                ))
                .branch(
                    case![GlobalState::CreatingDailyReminder(auth, x)]
                        .extract_auth_info::<CreatingDailyReminderState>()
                        .branch(
                            case![CreatingDailyReminderState::WaitingForReminderText]
                                .endpoint(receive_reminder_text),
                        )
                        .branch(
                            case![CreatingDailyReminderState::WaitingForFiringTime { text }]
                                .endpoint(receive_firing_time),
                        ),
                ),
        )
        .branch(
            Update::filter_callback_query().branch(
                case![GlobalState::CreatingDailyReminder(auth, x)]
                    .extract_auth_info::<CreatingDailyReminderState>()
                    .branch(
                        case![CreatingDailyReminderState::WaitingForConfirmation {
                            text,
                            firing_time
                        }]
                        .endpoint(confirm_reminder),
                    ),
            ),
        )
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use nadoeda_scheduler::ScheduledReminder;
    use nadoeda_storage::{
        ReminderStorage,
        sqlite::{self, reminder_storage::SqliteReminderStorage},
    };
    use sqlx::{Pool, Sqlite};
    use std::sync::Arc;
    use teloxide::{
        dispatching::dialogue::{self, InMemStorage},
        dptree::deps,
    };
    use teloxide_tests::{MockBot, MockMessageText};

    use super::*;

    struct NoopReminderScheduler;
    #[async_trait]
    impl ReminderScheduler for NoopReminderScheduler {
        async fn schedule_reminder(
            &self,
            schedule_request: ScheduleRequest,
        ) -> anyhow::Result<ScheduledReminder> {
            Ok(ScheduledReminder::new(1))
        }

        async fn cancel_reminder(
            &self,
            scheduled_reminder: &ScheduledReminder,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn acknowledge_reminder(
            &self,
            scheduled_reminder: &ScheduledReminder,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn confirm_reminder(
            &self,
            scheduled_reminder: &ScheduledReminder,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[sqlite::sqlx::test]
    async fn test(pool: Pool<Sqlite>) {
        let reminder_storage = Arc::new(SqliteReminderStorage::new(pool));
        let scheduler: Arc<dyn ReminderScheduler> = Arc::new(NoopReminderScheduler);
        let schema =
            dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mut bot = MockBot::new(MockMessageText::new().text("New Reminder"), schema);

        bot.dependencies(deps![
            reminder_storage,
            scheduler,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Authenticated(AuthenticationInfo(0))
        ]);

        bot.set_state(GlobalState::CreatingDailyReminder(
            AuthenticationInfo(0),
            CreatingDailyReminderState::WaitingForReminderText,
        ))
        .await;

        bot.dispatch_and_check_state(GlobalState::CreatingDailyReminder(
            AuthenticationInfo(0),
            CreatingDailyReminderState::WaitingForFiringTime {
                text: "New Reminder".to_string(),
            },
        ))
        .await;
    }
}
