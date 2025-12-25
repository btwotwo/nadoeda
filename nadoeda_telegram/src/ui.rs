mod authenticate_user;
mod create_daily_reminder;
mod edit_reminders;
mod util;

#[cfg(test)]
mod tests;

use authenticate_user::AuthenticationState;
use edit_reminders::EditingRemindersState;
use nadoeda_models::user::User;

use create_daily_reminder::CreatingDailyReminderState;
use dptree::case;
use nadoeda_scheduler::ReminderScheduler;
use nadoeda_storage::{
    ReminderStorage,
    sqlite::{reminder_storage::SqliteReminderStorage, user_storage::SqliteUserInfoStorage},
};
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue::{self, InMemStorage},
    macros::BotCommands,
    prelude::*,
};
use util::HandlerExtensions;

type GlobalDialogue = Dialogue<GlobalState, InMemStorage<GlobalState>>;
type AuthenticatedDialogue =
    Dialogue<AuthenticatedActionState, InMemStorage<AuthenticatedActionState>>;

type HandlerResult = anyhow::Result<()>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct AuthenticationInfo(User);

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum GlobalState {
    #[default]
    Unauthenticated,
    Authenticating(AuthenticationState),
    AuthenticatedV2(AuthenticationInfo, AuthenticatedActionState),
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum AuthenticatedActionState {
    #[default]
    Idle,
    CreatingDailyReminder(CreatingDailyReminderState),
    EditingReminder(EditingRemindersState),
}

pub struct TelegramInteractionInterface;

impl TelegramInteractionInterface {
    pub async fn start(
        bot: teloxide::Bot,
        scheduler: Arc<dyn ReminderScheduler>,
        reminder_storage: Arc<SqliteReminderStorage>,
        user_storage: Arc<SqliteUserInfoStorage>,
    ) {
        log::info!("Starting Telegram UI.");

        let invalid_state_handler =
            Update::filter_message().branch(dptree::endpoint(invalid_state));

        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
        .chain(authenticate_user::schema())
        .branch(
            case![GlobalState::AuthenticatedV2(auth, state)]
                .inject_auth_and_state::<AuthenticatedActionState>()
                .enter_dialogue::<Update, InMemStorage<AuthenticatedActionState>, AuthenticatedActionState>()
                .branch(get_cancel_handler::<AuthenticatedActionState>())
                .branch(create_daily_reminder::schema())
                .branch(edit_reminders::schema())
                .branch(get_invalid_callback_handler::<AuthenticatedActionState>())
        )
        .branch(get_cancel_handler::<GlobalState>())
        .branch(invalid_state_handler)
        .branch(get_invalid_callback_handler::<GlobalState>());

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![
                InMemStorage::<GlobalState>::new(),
                InMemStorage::<AuthenticatedActionState>::new(),
                scheduler,
                reminder_storage,
                user_storage
            ])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await
    }
}

fn get_invalid_callback_handler<S>()
-> Handler<'static, Result<(), anyhow::Error>, teloxide::dispatching::DpHandlerDescription>
where
    S: Send + Sync + Clone + 'static,
{
    Update::filter_callback_query().branch(dptree::endpoint(invalid_query::<S>))
}

fn get_cancel_handler<S>()
-> Handler<'static, Result<(), anyhow::Error>, teloxide::dispatching::DpHandlerDescription>
where
    S: Send + Sync + Clone + 'static,
{
    Update::filter_message().branch(
        teloxide::filter_command::<GlobalCommand, _>()
            .branch(case![GlobalCommand::Cancel].endpoint(cancel::<S>)),
    )
}
async fn cancel<S>(
    bot: Bot,
    dialogue: Dialogue<S, InMemStorage<S>>,
    msg: Message,
) -> HandlerResult
where
    S: Send + Sync + Clone + 'static,
{
    bot.send_message(msg.chat.id, "Cancelled current operation.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
    msg.chat.id,
    "Unable to handle the message. Please try again or use /cancel to stop current operation.",
)
.await?;
    Ok(())
}

async fn invalid_query<S>(
    bot: Bot,
    dialogue: Dialogue<S, InMemStorage<S>>,
    query: CallbackQuery,
) -> HandlerResult
where
    S: Send + Clone + 'static,
{
    bot.answer_callback_query(query.id).await?;
    bot.send_message(
    dialogue.chat_id(),
    "Unable to handle the query result. Please try again or use /cancel to stop current operation.",
).await?;

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum GlobalCommand {
    ListReminders,
    CreateReminder,
    Cancel,
}
