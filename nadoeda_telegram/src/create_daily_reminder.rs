use std::sync::Arc;

use chrono::NaiveTime;
use dptree::case;
use nadoeda_scheduler::{ReminderScheduler, ScheduleRequest};
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{Bot, types::Message};

use nadoeda_models::reminder::{Reminder, ReminderFireTime, ReminderState};
use nadoeda_storage::NewReminder;

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
    msg: Message,
) -> HandlerResult {
    bot.send_message(
            msg.chat.id,
            "Creating a new daily reminder! Please input reminder text. If you want to cancel, use the /cancel command.",
        )
        .await?;

    dialogue
        .update(GlobalState::CreatingDailyReminder(
            CreatingDailyReminderState::WaitingForReminderText,
        ))
        .await?;

    Ok(())
}
async fn receive_reminder_text(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let message = format!(
                "Great! You will be reminded about \"{}\"\nNow, please enter time when reminder is going to be fired (e.g. 13:00)",
                teloxide::utils::markdown::escape(text)
            );
            bot.send_message(msg.chat.id, message).await?;
            dialogue
                .update(GlobalState::CreatingDailyReminder(
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
    // storage: HandlerReminderStorageType,
    bot: Bot,
    dialogue: GlobalDialogue,
    (text, firing_time): (String, NaiveTime),
    query: CallbackQuery,
    scheduler: Arc<dyn ReminderScheduler>,
) -> HandlerResult {
    let reminder = NewReminder {
        text,
        fire_at: ReminderFireTime::new(firing_time),
    };

    // let reminder_id = storage.insert(reminder).await?;
    let reminder_id = 1;
    bot.answer_callback_query(query.id).await?;

    log::info!("Created reminder with id {}", reminder_id);

    let reminder = Reminder {
        id: reminder_id,
        text: "Rroro".to_string(),
        state: ReminderState::Pending,
        fire_at: ReminderFireTime::new(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
    };
    // let reminder = storage
    //     .get(reminder_id)
    //     .await
    //     .expect("Reminder was just created.");

    scheduler
        .schedule_reminder(ScheduleRequest::new(reminder))
        .await;

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
                    case![GlobalState::Idle].branch(
                        case![GlobalCommand::CreateReminder].endpoint(create_daily_reminder_start),
                    ),
                ))
                .branch(
                    case![GlobalState::CreatingDailyReminder(x)]
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
                case![GlobalState::CreatingDailyReminder(x)].branch(
                    case![CreatingDailyReminderState::WaitingForConfirmation { text, firing_time }]
                        .endpoint(confirm_reminder),
                ),
            ),
        )
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use nadoeda_scheduler::ScheduledReminder;
    use nadoeda_storage::{InMemoryReminderStorage, ReminderStorage};
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

    #[tokio::test]
    async fn test() {
        let reminder_storage: Arc<dyn ReminderStorage> = Arc::new(InMemoryReminderStorage::new());
        let scheduler: Arc<dyn ReminderScheduler> = Arc::new(NoopReminderScheduler);
        let schema =
            dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mut bot = MockBot::new(MockMessageText::new().text("New Reminder"), schema);

        bot.dependencies(deps![
            reminder_storage,
            scheduler,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Idle
        ]);

        bot.set_state(GlobalState::CreatingDailyReminder(
            CreatingDailyReminderState::WaitingForReminderText,
        ))
        .await;

        bot.dispatch_and_check_state(GlobalState::CreatingDailyReminder(
            CreatingDailyReminderState::WaitingForFiringTime {
                text: "New Reminder".to_string(),
            },
        ))
        .await;
    }
}
