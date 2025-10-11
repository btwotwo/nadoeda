use async_trait::async_trait;
use nadoeda_models::reminder::Reminder;
use nadoeda_scheduler::{ReminderDeliveryChannel, ReminderMessageType};
use teloxide::prelude::*;

pub struct TelegramDeliveryChannel {
    bot: Bot,
    chat_id: ChatId
}

impl TelegramDeliveryChannel {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self {
            bot,
            chat_id
        }
    }

    pub async fn send_message(&self, msg: &str, chat_id: ChatId) -> anyhow::Result<()> {
        self.bot.send_message(chat_id, msg).await?;
        Ok(())
    }
}

#[async_trait]
impl ReminderDeliveryChannel for TelegramDeliveryChannel {
    async fn send_reminder_notification(&self, reminder: &Reminder, message: ReminderMessageType) {
        let message_txt = format!("{} - {:?}", reminder.text, message);
        self.bot.send_message(self.chat_id, message_txt).await.unwrap();
    }
}
