use std::sync::Arc;

use dptree::case;
use nadoeda_storage::reminder;
use nadoeda_storage::{ReminderStorage, sqlite::reminder_storage::SqliteReminderStorage};
use teloxide::dispatching::dialogue;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::utils::markdown;
use teloxide::{dispatching::UpdateHandler, macros::BotCommands};
use teloxide::{filter_command, prelude::*};

use nadoeda_models::reminder::{Reminder, ReminderId};

use crate::{AuthenticatedActionState, AuthenticatedDialogue, AuthenticationInfo};

use super::{GlobalCommand, GlobalDialogue, HandlerResult};

#[derive(Clone, Default)]
pub(super) enum EditingRemindersState {
    #[default]
    Start,
    WaitingForFieldSelection(Arc<Reminder>),
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    parse_with = "split",
    command_separator = "_"
)]
enum EditReminderCommand {
    Edit(ReminderId),
}

async fn list_reminders(
    storage: Arc<SqliteReminderStorage>,
    bot: Bot,
    dialogue: GlobalDialogue,
    auth: AuthenticationInfo,
    msg: Message,
) -> HandlerResult {
    let reminders = storage.get_all_user_reminders(&auth.0.id).await?;
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

async fn edit_reminder(
    id: ReminderId,
    msg: Message,
    dialogue: AuthenticatedDialogue,
    auth: AuthenticationInfo,
    store: Arc<SqliteReminderStorage>,
    bot: Bot,
) -> HandlerResult {
    let reminder = store.get(&id, &auth.0.id).await?;
    if let Some(reminder) = reminder {
        let text_button = InlineKeyboardButton::callback("Text", "Text");
        let time_button = InlineKeyboardButton::callback("Time", "Time");
        let keyboard = InlineKeyboardMarkup::new(vec![vec![text_button, time_button]]);

        bot.send_message(msg.chat.id, "What do you want to update?")
            .reply_markup(keyboard)
            .await?;

        dialogue
            .update(AuthenticatedActionState::EditingReminder(
                EditingRemindersState::WaitingForFieldSelection(Arc::new(reminder)),
            ))
            .await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "Invalid Edit Reminder command. Please try again.",
        )
        .await?;
    }

    Ok(())
}

async fn handle_selected_field(
    msg: Message,
    dialogue: AuthenticatedDialogue,
    auth: AuthenticationInfo,
    store: Arc<SqliteReminderStorage>,
    bot: Bot,
    query: CallbackQuery
) -> HandlerResult {
    
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
    dptree::entry()
        .branch(
            case![AuthenticatedActionState::Idle].branch(
                Update::filter_message()
                    .branch(
                        filter_command::<GlobalCommand, _>()
                            .branch(case![GlobalCommand::ListReminders].endpoint(list_reminders)),
                    )
                    .branch(
                        filter_command::<EditReminderCommand, _>()
                            .branch(case![EditReminderCommand::Edit(id)].endpoint(edit_reminder)),
                    ),
            ),
        )
        .branch(
            case![AuthenticatedActionState::EditingReminder(x)].branch(
                Update::filter_callback_query().branch(
                    case![EditingRemindersState::WaitingForFieldSelection(rem)].endpoint(f),
                ),
            ),
        )
}
