use teloxide::prelude::*;

use crate::appsettings;

pub struct TelegramDeliveryChannel {
    bot: Bot,
}

impl TelegramDeliveryChannel {
    pub fn create() -> Self {
        let bot = Bot::new(appsettings::get().telegram.token.clone());

        Self { bot }
    }

    pub async fn send_message(&self, msg: &str, chat_id: ChatId) -> anyhow::Result<()> {
        self.bot.send_message(chat_id, msg).await?;
        Ok(())
    }
}
