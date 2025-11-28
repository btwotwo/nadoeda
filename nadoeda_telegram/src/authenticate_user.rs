use std::sync::Arc;

use chrono::TimeZone;
use dptree::case;
use nadoeda_models::chrono_tz;
use nadoeda_models::user::User;
use nadoeda_storage::sqlite::user_storage::SqliteUserInfoStorage;
use nadoeda_storage::{NewUser, UserInfoStorage};
use teloxide::prelude::*;
use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    dptree, handler,
    types::Update,
};

use crate::{
    AuthenticatedActionState, AuthenticationInfo, GlobalCommand, GlobalDialogue, GlobalState,
    HandlerResult,
};

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum AuthenticationState {
    #[default]
    Start,
    WaitingForTimezone,
}

async fn try_authenticate(
    bot: Bot,
    dialogue: GlobalDialogue,
    msg: Message,
    user_store: Arc<SqliteUserInfoStorage>,
) -> HandlerResult {
    let user = user_store.get_by_tg_chat(msg.chat.id.0).await?;

    if let Some(user) = user {
        bot.send_message(msg.chat.id, "I've seen you before. Proceed.")
            .await?;

        dialogue
            .update(GlobalState::AuthenticatedV2(
                AuthenticationInfo(user),
                crate::AuthenticatedActionState::Idle,
            ))
            .await?;
    } else {
        bot.send_message(msg.chat.id, "Not seen before. Send me your timezone.")
            .await?;

        dialogue
            .update(GlobalState::Authenticating(
                AuthenticationState::WaitingForTimezone,
            ))
            .await?;
    }

    Ok(())
}

pub async fn get_user_timezone(
    bot: Bot,
    dialogue: GlobalDialogue,
    msg: Message,
    user_store: Arc<SqliteUserInfoStorage>,
) -> HandlerResult {
    if let Some(tz_str) = msg.text() {
        match tz_str.parse::<chrono_tz::Tz>() {
            Ok(timezone) => {
                let user = user_store
                    .create(NewUser {
                        timezone,
                        tg_chat_id: Some(msg.chat.id.0),
                    })
                    .await?;

                bot.send_message(msg.chat.id, "Timezone received. Welcome aboard.")
                    .await?;

                dialogue
                    .update(GlobalState::AuthenticatedV2(
                        AuthenticationInfo(user),
                        AuthenticatedActionState::Idle,
                    ))
                    .await?;
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Invalid timezone. Please try again.")
                    .await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "Invalid.").await?;
    };

    Ok(())
}

pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    Update::filter_message()
        .branch(case![GlobalState::Unauthenticated].endpoint(try_authenticate))
        .branch(
            case![GlobalState::Authenticating(x)]
                .branch(case![AuthenticationState::WaitingForTimezone].endpoint(get_user_timezone)),
        )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{AuthenticationInfo, test_utils::*};

    use super::*;

    use nadoeda_models::chrono_tz;
    use nadoeda_storage::NewUser;
    use sqlx::{Pool, Sqlite};
    use teloxide::{
        dispatching::dialogue::{self, InMemStorage},
        dptree::deps,
    };
    use teloxide_tests::{MockBot, MockMessageText};

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
            AuthenticationState::WaitingForTimezone,
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

        let user = user_storage
            .create(NewUser {
                timezone: chrono_tz::Tz::Europe__Prague,
                tg_chat_id: Some(chat_id.0),
            })
            .await
            .unwrap();

        let mut bot = MockBot::new(MockMessageText::new().text("Random Text"), schema);

        bot.dependencies(deps![
            storage,
            user_storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated
        ]);

        bot.set_state(GlobalState::Unauthenticated).await;

        bot.dispatch_and_check_state(GlobalState::AuthenticatedV2(
            AuthenticationInfo(user),
            crate::AuthenticatedActionState::Idle,
        ))
        .await
    }

    #[sqlx::test(migrations = "../nadoeda_storage/migrations")]
    async fn given_provided_correct_timezone_should_set_authenticated_state(pool: Pool<Sqlite>) {
        let storage = storage(pool.clone());
        let user_storage = user_storage(pool.clone());
        let schema =
            dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mock_message = MockMessageText::new().text("Europe/Prague");
        let chat_id = mock_message.chat.id;

        let mut bot = MockBot::new(mock_message, schema);

        bot.dependencies(deps![
            storage,
            user_storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated
        ]);

        bot.set_state(GlobalState::Authenticating(
            AuthenticationState::WaitingForTimezone,
        ))
        .await;

        bot.dispatch_and_check_state(GlobalState::AuthenticatedV2(
            AuthenticationInfo(User {
                id: 1,
                timezone: chrono_tz::Tz::Europe__Prague,
                tg_chat_id: Some(chat_id.0),
            }),
            AuthenticatedActionState::Idle,
        ))
        .await;
    }
}
