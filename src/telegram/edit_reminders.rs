use dptree::case;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::markdown;

use crate::reminder::Reminder;

use super::{
    GlobalCommand, GlobalDialogue, GlobalState, HandlerReminderStorageType, HandlerResult,
};

#[derive(Clone, Default)]
pub(super) enum EditRemindersState {
    #[default]
    Start,
}

async fn list_reminders(
    storage: HandlerReminderStorageType,
    bot: Bot,
    dialogue: GlobalDialogue,
    msg: Message,
) -> HandlerResult {
    let reminders = storage.get_all().await;
    let message = if reminders.is_empty() {
        "You have to create at least one reminder\\!".to_string()
    } else {
        reminders
            .iter()
            .enumerate()
            .map(|(i, reminder)| display_reminder(i + 1, reminder))
            .collect::<Vec<String>>()
            .join("\n\n")
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

fn display_reminder(order: usize, reminder: &Reminder) -> String {
    format!(
        "{order}: *{0}* \\(remind every day at *{1}*\\)
Edit \\- /edit\\_{2}",
        markdown::escape(&reminder.text),
        reminder.fire_at.time().format("%H:%M"),
        reminder.id
    )
}

pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry().branch(
        Update::filter_message().branch(
            teloxide::filter_command::<GlobalCommand, _>().branch(
                case![GlobalState::Idle]
                    .branch(case![GlobalCommand::ListReminders].endpoint(list_reminders)),
            ),
        ),
    )
}
