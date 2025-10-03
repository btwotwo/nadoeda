use teloxide::prelude::*;

pub struct TelegramDeliveryChannel {
    bot: Bot,
}

impl TelegramDeliveryChannel {
    pub fn create(token: String) -> Self {
        let bot = Bot::new(token);

        Self { bot }
    }

    pub async fn send_message(&self, msg: &str, chat_id: ChatId) -> anyhow::Result<()> {
        self.bot.send_message(chat_id, msg).await?;
        Ok(())
    }
}
