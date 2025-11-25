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
    WaitingForUserInfo
}

async fn try_authenticate(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    todo!()
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

    use nadoeda_storage::sqlite::reminder_storage::SqliteReminderStorage;
    use sqlx::{Pool, Sqlite};
    use teloxide::{dispatching::dialogue::{self, InMemStorage}, dptree::deps};
    use teloxide_tests::{MockBot, MockMessageText};

    #[sqlx::test]
    async fn given_user_not_exist_should_ask_for_info(pool: Pool<Sqlite>) {
        let storage = Arc::new(SqliteReminderStorage::new(pool));
        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>().branch(schema());
        let mut bot = MockBot::new(MockMessageText::new().text("Random Text"), schema);

        bot.dependencies(deps![
            storage,
            InMemStorage::<GlobalState>::new(),
            GlobalState::Unauthenticated
        ]);

        bot.set_state(GlobalState::Unauthenticated).await;

        bot.dispatch_and_check_state(GlobalState::Authenticating(AuthenticationState::WaitingForUserInfo)).await;
    }
}
