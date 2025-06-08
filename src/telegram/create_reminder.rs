use chrono::NaiveTime;
use dptree::case;
use teloxide::dispatching::UpdateHandler;
use teloxide::dispatching::dialogue::{self, InMemStorage};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use teloxide::{Bot, handler, types::Message};

use super::{GlobalCommand, GlobalDialogue, GlobalState, HandlerResult};

#[derive(Clone, Default)]
pub(super) enum CreateReminderState {
    #[default]
    Start,
    ReceiveText,
    ReceiveFiringTime {
        text: String,
    },
    ReceiveFiringPeriod {
        text: String,
        firing_time: NaiveTime,
    },
}

async fn create_reminder_start(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
            msg.chat.id,
            "Creating a new reminder! Please input reminder text. If you want to cancel, use the /cancel command.",
        )
        .await?;

    dialogue
        .update(GlobalState::CreateReminder(
            CreateReminderState::ReceiveText,
        ))
        .await?;

    Ok(())
}
async fn receive_reminder_text(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let message = format!(
                "Great! You will be reminded about \"{}\"\nNow, please enter time when reminder is going to be fired (e.g. 13:00)",
                text
            );
            bot.send_message(msg.chat.id, message).await?;
            dialogue
                .update(GlobalState::CreateReminder(
                    CreateReminderState::ReceiveFiringTime {
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
            bot.send_message(
                msg.chat.id,
                format!("Great! Your reminder will fire at {}.\n\nNow, please select how often reminder is going to be fired.", time),
            )
                .await?;

            dialogue
                .update(GlobalState::CreateReminder(
                    CreateReminderState::ReceiveFiringPeriod {
                        text,
                        firing_time: time,
                    },
                ))
                .await?;
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "Could not parse time. Please send time in the following format: \"13:00\"",
            )
            .await?;
        }
    }
    Ok(())
}

async fn receive_firing_period(
    bot: Bot,
    dialogue: GlobalDialogue,
    text: String,
    firing_time: NaiveTime,
    msg: Message
) -> HandlerResult {
    Ok(())
}

fn prepare_firing_period_keyboard() {
    
}

pub fn schema() -> UpdateHandler<anyhow::Error> {
    Update::filter_message()
        .branch(
            teloxide::filter_command::<GlobalCommand, _>().branch(
                case![GlobalState::Idle]
                    .branch(case![GlobalCommand::CreateReminder].endpoint(create_reminder_start)),
            ),
        )
        .branch(
            case![GlobalState::CreateReminder(x)]
                .branch(case![CreateReminderState::ReceiveText].endpoint(receive_reminder_text))
                .branch(
                    case![CreateReminderState::ReceiveFiringTime { text }]
                        .endpoint(receive_firing_time),
                )
                .branch(case![CreateReminderState::ReceiveFiringPeriod { text, firing_time }].endpoint(receive_firing_period)),
        )
}
