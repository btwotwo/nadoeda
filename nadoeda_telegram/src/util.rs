use teloxide::{
    dptree::{Handler, HandlerDescription}, payloads::EditMessageReplyMarkupSetters, sugar::bot::BotMessagesExt, types::{CallbackQuery, InlineKeyboardMarkup, MaybeInaccessibleMessage, Message}, Bot
};

use crate::AuthenticationInfo;

pub trait HandlerExtensions<'a, Output, Descr>
where
    Output: 'a,
    Descr: HandlerDescription,
{
    fn inject_auth_and_state<TState>(self) -> Handler<'a, Output, Descr>
    where
        TState: Clone + Send + Sync + 'static;
}

impl<'a, Output, Descr> HandlerExtensions<'a, Output, Descr> for Handler<'a, Output, Descr>
where
    Output: 'a,
    Descr: HandlerDescription,
{
    fn inject_auth_and_state<TState>(self) -> Handler<'a, Output, Descr>
    where
        TState: Clone + Send + Sync + 'static,
    {
        self.map(|(a, _): (AuthenticationInfo, TState)| a)
            .map(|(_, b): (AuthenticationInfo, TState)| b)
    }
}

pub fn try_get_message_from_query(query: &CallbackQuery) -> Option<&Message> {
    query.message.as_ref().and_then(|msg| match msg {
        MaybeInaccessibleMessage::Inaccessible(_) => None,
        MaybeInaccessibleMessage::Regular(message) => Some(message.as_ref()),
    })
}

pub async fn clear_message_buttons(bot: &Bot, message: &Message) -> Result<(), anyhow::Error> {
    bot.edit_reply_markup(message)
        .reply_markup(InlineKeyboardMarkup::default())
        .await?;

    Ok(())
}
