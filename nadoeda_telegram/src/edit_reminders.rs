use std::sync::Arc;

use dptree::case;
use nadoeda_storage::{ReminderStorage, sqlite::reminder_storage::SqliteReminderStorage};
use teloxide::{dispatching::UpdateHandler, macros::BotCommands};
use teloxide::{filter_command, prelude::*};
use teloxide::types::ParseMode;
use teloxide::utils::markdown;

use nadoeda_models::reminder::{Reminder, ReminderId};

use crate::AuthenticatedActionState;

use super::{GlobalCommand, GlobalDialogue, HandlerResult};

#[derive(Clone, Default)]
pub(super) enum EditRemindersState {
    #[default]
    Start,
}


#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    parse_with = "split",
    command_separator = "_"
)]
enum EditReminderCommand {
    Edit(ReminderId)
}

async fn list_reminders(
    storage: Arc<SqliteReminderStorage>,
    bot: Bot,
    dialogue: GlobalDialogue,
    msg: Message,
) -> HandlerResult {
    let reminders = storage.get_all_user_reminders(&0).await?;
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

async fn edit_reminder(id: ReminderId, msg: Message, bot: Bot) -> HandlerResult {
    bot.send_message(msg.chat.id, format!("Editing reminder {id}")).await?;
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
        case![AuthenticatedActionState::Idle].branch(
            Update::filter_message().branch(
                filter_command::<GlobalCommand, _>()
                    .branch(case![GlobalCommand::ListReminders].endpoint(list_reminders)),
            ).branch(
                filter_command::<EditReminderCommand, _>().branch(
                    case![EditReminderCommand::Edit(id)].endpoint(edit_reminder)
                )
            ),
        ),
    )
}
