use dptree::case;
use teloxide::prelude::*;
use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    dptree, handler,
    types::Update,
};

use crate::{GlobalCommand, GlobalDialogue, GlobalState, HandlerResult};

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum AuthenticationState {
    #[default]
    Start,
    WaitingForUserInfo,
}

async fn try_authenticate(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Not seen before. Send me something")
        .await?;

    dialogue
        .update(GlobalState::Authenticating(
            AuthenticationState::WaitingForUserInfo,
        ))
        .await?;

    Ok(())
}

pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry().branch(
        Update::filter_message()
            .branch(case![GlobalState::Unauthenticated].endpoint(try_authenticate)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    use nadoeda_models::chrono_tz;
    use nadoeda_storage::{sqlite::{reminder_storage::SqliteReminderStorage, user_storage::{self, SqliteUserInfoStorage}}, NewUser, UserInfoStorage};
    use sqlx::{Pool, Sqlite};
    use teloxide::{
        dispatching::dialogue::{self, InMemStorage},
        dptree::deps,
    };
    use teloxide_tests::{MockBot, MockMessageText};

    fn storage(pool: Pool<Sqlite>) -> Arc<SqliteReminderStorage> {
        Arc::new(SqliteReminderStorage::new(pool))
    }

    fn user_storage(pool: Pool<Sqlite>) -> Arc<SqliteUserInfoStorage> {
        Arc::new(SqliteUserInfoStorage::new(pool))
    }

    #[sqlx::test(migrations = "../nadoeda_storage/migrations")]
    async fn given_user_not_exist_should_ask_for_info(pool: Pool<Sqlite>) {
        let storage = storage(pool.clone());
        let user_storage = user_storage(pool.clone());
        let schema =
            dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mut bot = MockBot::new(MockMessageText::new().text("Random Text"), schema);

        bot.dependencies(deps![
            storage,
            user_storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated
        ]);

        bot.set_state(GlobalState::Unauthenticated).await;

        bot.dispatch_and_check_state(GlobalState::Authenticating(
            AuthenticationState::WaitingForUserInfo,
        ))
        .await;
    }


    #[sqlx::test(migrations = "../nadoeda_storage/migrations")]
    async fn given_user_exists_should_not_ask_for_info(pool: Pool<Sqlite>) {
        let storage = storage(pool.clone());
        let user_storage = user_storage(pool.clone());
        let schema =
            dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mock_message = MockMessageText::new().text("Random Text");
        let chat_id = mock_message.chat.id;

        user_storage.create(NewUser { timezone: chrono_tz::Tz::Europe__Prague, tg_chat_id: Some(chat_id.0) }).await.unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("Random Text"), schema);

        bot.dependencies(deps![
            storage,
            user_storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated
        ]);

        bot.set_state(GlobalState::Unauthenticated).await;
        
    }
}
