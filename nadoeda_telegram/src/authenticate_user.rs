use std::sync::Arc;

use nadoeda_models::chrono_tz;
use nadoeda_storage::sqlite::user_storage::SqliteUserInfoStorage;
use nadoeda_storage::{NewUser, UserInfoStorage};
use sqlx::Result;
use teloxide::dispatching::dialogue::InMemStorageError;
use teloxide::prelude::*;
use teloxide::types::UpdateKind;
use teloxide::{dispatching::UpdateHandler, dptree, types::Update};

use anyhow::anyhow;

use crate::{
    AuthenticatedActionState, AuthenticationInfo, GlobalDialogue, GlobalState, HandlerResult,
};
use thiserror::Error;

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) enum AuthenticationState {
    #[default]
    Start,
    WaitingForTimezone,
}

#[derive(Debug, Error)]
enum AuthError {
    #[error(transparent)]
    Telegram(#[from] teloxide::RequestError),

    #[error(transparent)]
    StorageError(#[from] teloxide::dispatching::dialogue::InMemStorageError),

    #[error(transparent)]
    Common(#[from] anyhow::Error),
}

impl Clone for AuthError {
    fn clone(&self) -> Self {
        match self {
            Self::Telegram(arg0) => Self::Telegram(arg0.clone()),
            Self::StorageError(_) => Self::StorageError(InMemStorageError::DialogueNotFound),
            Self::Common(error) => Self::Common(anyhow!(error.to_string())),
        }
    }
}

async fn try_authenticate(
    bot: Bot,
    dialogue: &GlobalDialogue,
    msg: Message,
    user_store: Arc<SqliteUserInfoStorage>,
) -> Result<bool, AuthError> {
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
        Ok(true)
    } else {
        bot.send_message(msg.chat.id, "Not seen before. Send me your timezone.")
            .await?;

        dialogue
            .update(GlobalState::Authenticating(
                AuthenticationState::WaitingForTimezone,
            ))
            .await?;

        Ok(false)
    }
}

pub async fn get_user_timezone(
    bot: Bot,
    dialogue: &GlobalDialogue,
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

async fn middleware(
    bot: Bot,
    dialogue: GlobalDialogue,
    state: GlobalState,
    update: Update,
    user_store: Arc<SqliteUserInfoStorage>,
) -> Result<Option<GlobalState>, AuthError> {
    let UpdateKind::Message(msg) = update.kind else {
        return Ok(Some(state));
    };

    match state {
        GlobalState::Unauthenticated => {
            let user_exists = try_authenticate(bot, &dialogue, msg, user_store).await?;

            if user_exists {
                let new_state = dialogue.get_or_default().await?;
                Ok(Some(new_state))
            } else {
                Ok(None)
            }
        }
        GlobalState::Authenticating(AuthenticationState::WaitingForTimezone) => {
            get_user_timezone(bot, &dialogue, msg, user_store).await?;
            Ok(None)
        }
        _ => Ok(Some(state)),
    }
}

pub(super) fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry()
        .map_async(middleware)
        .filter_map(|res: Result<Option<GlobalState>, AuthError>| res.ok().flatten())
        .inspect(|s: GlobalState| log::info!("Global state: {s:?}"))
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        sync::{Arc, atomic::AtomicBool},
    };

    use crate::{AuthenticationInfo, test_utils::*};

    use super::*;

    use nadoeda_models::{chrono_tz, user::User};
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

    #[sqlx::test(migrations = "../nadoeda_storage/migrations")]
    async fn given_unauthenticated_state_when_user_exists_should_pass_message_to_other_handlers(
        pool: Pool<Sqlite>,
    ) {
        let storage = storage(pool.clone());
        let user_storage = user_storage(pool.clone());
        let endpoint_called = Arc::new(AtomicBool::new(false));
        let clone = endpoint_called.clone();
        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
            .branch(schema())
            .endpoint(async move |flag: Arc<AtomicBool>| {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            });
        let mock_message = MockMessageText::new().text("Europe/Prague");
        let chat_id = mock_message.chat.id;

        let mut bot = MockBot::new(MockMessageText::new().text("Random Text"), schema);

        bot.dependencies(deps![
            storage,
            user_storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated,
            endpoint_called.clone()
        ]);

        bot.set_state(GlobalState::Unauthenticated).await;

        bot.dispatch_and_check_state(GlobalState::Authenticating(
            AuthenticationState::WaitingForTimezone,
        ))
            .await;

        assert!(endpoint_called.load(std::sync::atomic::Ordering::Acquire));
    }
}
