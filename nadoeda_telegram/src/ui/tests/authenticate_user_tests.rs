use crate::ui::{
    AuthenticatedActionState, AuthenticationInfo, AuthenticationState, GlobalState,
    tests::test_utils::*,
};

use crate::ui::authenticate_user::schema;
use nadoeda_models::{chrono_tz, user::User};
use nadoeda_storage::{NewUser, UserInfoStorage};
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{
        UpdateFilterExt,
        dialogue::{self, InMemStorage},
    },
    dptree::deps,
    types::Update,
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
        InMemStorage::<GlobalState>::new()
    ]);

    bot.set_state(GlobalState::Unauthenticated).await;

    bot.dispatch_and_check_state(GlobalState::AuthenticatedV2(
        AuthenticationInfo(user),
        AuthenticatedActionState::Idle,
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
        InMemStorage::<GlobalState>::new()
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
async fn given_unauthenticated_state_when_user_does_not_exist_should_not_pass_message_to_handlers(
    pool: Pool<Sqlite>,
) {
    let storage = storage(pool.clone());
    let user_storage = user_storage(pool.clone());
    let marker = CallMarker::new();
    let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
        .chain(schema())
        .branch(Update::filter_message().endpoint(call_marker_endpoint));

    let (mut bot, _) = bot("Random", schema);

    bot.dependencies(deps![
        storage,
        user_storage,
        InMemStorage::<GlobalState>::new(),
        marker.clone()
    ]);

    bot.set_state(GlobalState::Unauthenticated).await;

    bot.dispatch_and_check_state(GlobalState::Authenticating(
        AuthenticationState::WaitingForTimezone,
    ))
    .await;

    assert!(!marker.was_called());
}

#[sqlx::test(migrations = "../nadoeda_storage/migrations")]
async fn given_unauthenticated_state_when_user_exists_should_pass_message_to_handlers(
    pool: Pool<Sqlite>,
) {
    let storage = storage(pool.clone());
    let user_storage = user_storage(pool.clone());
    let marker = CallMarker::new();
    let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
        .chain(schema())
        .branch(Update::filter_message().endpoint(call_marker_endpoint));

    let (mut bot, chat_id) = bot("Random", schema);

    let user = user_storage
        .create(NewUser {
            timezone: chrono_tz::Tz::Europe__Prague,
            tg_chat_id: Some(chat_id.0),
        })
        .await
        .expect("Error creating user");

    bot.dependencies(deps![
        storage,
        user_storage,
        InMemStorage::<GlobalState>::new(),
        marker.clone()
    ]);

    bot.set_state(GlobalState::Unauthenticated).await;

    bot.dispatch_and_check_state(GlobalState::AuthenticatedV2(
        AuthenticationInfo(user),
        AuthenticatedActionState::Idle,
    ))
    .await;

    assert!(marker.was_called());
}
