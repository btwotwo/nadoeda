use std::sync::Arc;

use async_trait::async_trait;
use nadoeda_models::{reminder::Reminder, user::UserId};
use nadoeda_scheduler::delivery::{ReminderDeliveryChannel, ReminderMessageType};
use nadoeda_storage::{UserInfoStorage, sqlite::user_storage::SqliteUserInfoStorage};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup, ParseMode},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelegramDeliveryChannelError {
    #[error(transparent)]
    Common(#[from] anyhow::Error),

    #[error(transparent)]
    Telegram(#[from] teloxide::RequestError),

    #[error("UserId in Reminder is invalid {0}")]
    InvalidUser(UserId),

    #[error("User does not have Telegram chat id configured {0}")]
    NoTelegramConfigured(UserId),
}

pub struct TelegramDeliveryChannel {
    user_store: Arc<SqliteUserInfoStorage>,
    bot: Bot,
}

impl TelegramDeliveryChannel {
    pub fn new(user_store: Arc<SqliteUserInfoStorage>, bot: Bot) -> Self {
        Self { user_store, bot }
    }
}

#[async_trait]
impl ReminderDeliveryChannel for TelegramDeliveryChannel {
    async fn send_reminder_notification(
        &self,
        reminder: &Reminder,
        message: ReminderMessageType,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let user = self
            .user_store
            .get(&reminder.user_id)
            .await?
            .ok_or(TelegramDeliveryChannelError::InvalidUser(reminder.user_id))?;

        let chat_id = user
            .tg_chat_id
            .ok_or(TelegramDeliveryChannelError::NoTelegramConfigured(
                reminder.user_id,
            ))?;

        let message_text = get_message_text(reminder, message);
        let keyboard_markup = get_keyboard_markup(reminder, message);

        self.bot
            .send_message(ChatId(chat_id), message_text)
            .reply_markup(keyboard_markup)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }
}

fn get_keyboard_markup(reminder: &Reminder, message: ReminderMessageType) -> InlineKeyboardMarkup {
    match message {
        ReminderMessageType::Fired
        | ReminderMessageType::Nag
        | ReminderMessageType::Confirmation => {
            let confirm_button = InlineKeyboardButton::callback("Confirm", reminder.id.to_string());
            InlineKeyboardMarkup::new(vec![vec![confirm_button]])
        }
        _ => InlineKeyboardMarkup::new(vec![vec![]]),
    }
}

fn get_message_text(reminder: &Reminder, message: ReminderMessageType) -> String {
    match message {
        ReminderMessageType::Scheduled => format!("⏱️: Scheduled *{}*.", reminder.text),
        ReminderMessageType::Fired => format!("🚨: {}", reminder.text),
        ReminderMessageType::Nag => format!("🚨 (nag): {}", reminder.text),
        ReminderMessageType::Confirmation => format!("⁉️: {}", reminder.text),
        ReminderMessageType::Acknowledge => format!("☑️: {}", reminder.text),
        ReminderMessageType::Timeout => format!("No reaction! Stopping."),
        ReminderMessageType::Finished => format!("✅: {}", reminder.text),
        ReminderMessageType::Cancelled => format!("❌: Cancelled {}", reminder.text),
    }
}
