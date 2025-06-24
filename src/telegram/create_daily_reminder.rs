use std::sync::Arc;

use chrono::NaiveTime;
use dptree::case;
use teloxide::dispatching::UpdateHandler;
use teloxide::dispatching::dialogue::{self, GetChatId, InMemStorage};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{Bot, handler, types::Message};

use crate::reminder::{Reminder, ReminderFireTime};
use crate::scheduling::ReminderManagerTrait;
use crate::storage::{InMemoryReminderStorage, NewReminder, ReminderStorage};

use super::{GlobalCommand, GlobalDialogue, GlobalState, HandlerResult};

#[derive(Clone, Default)]
pub(super) enum CreateDailyReminderState {
    #[default]
    Start,
    ReceiveText,
    ReceiveFiringTime {
        text: String,
    },
    Confirm {
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
        .update(GlobalState::CreateDailyReminder(
            CreateDailyReminderState::ReceiveText,
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
                .update(GlobalState::CreateDailyReminder(
                    CreateDailyReminderState::ReceiveFiringTime {
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
                .update(GlobalState::CreateDailyReminder(
                    CreateDailyReminderState::Confirm {
                        text,
                        firing_time: time,
                    },
                ))
                .await?;

            bot.send_message(msg.chat.id, message_txt)
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

async fn save_reminder(
    storage: Arc<dyn ReminderStorage + Send + Sync>,
    bot: Bot,
    dialogue: GlobalDialogue,
    (text, firing_time): (String, NaiveTime),
    query: CallbackQuery,
) -> HandlerResult {
    let reminder = NewReminder {
        text,
        fire_at: ReminderFireTime::new(firing_time),
    };

    storage.insert(reminder).await?;
    bot.answer_callback_query(&query.id).await?;

    bot.send_message(query.chat_id().unwrap(), "Reminder saved!")
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
                    case![GlobalState::CreateDailyReminder(x)]
                        .branch(
                            case![CreateDailyReminderState::ReceiveText]
                                .endpoint(receive_reminder_text),
                        )
                        .branch(
                            case![CreateDailyReminderState::ReceiveFiringTime { text }]
                                .endpoint(receive_firing_time),
                        ),
                ),
        )
        .branch(
            Update::filter_callback_query().branch(
                case![GlobalState::CreateDailyReminder(x)].branch(
                    case![CreateDailyReminderState::Confirm { text, firing_time }]
                        .endpoint(save_reminder),
                ),
            ),
        )
}
