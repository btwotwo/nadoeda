mod create_daily_reminder;
mod edit_reminders;
mod authenticate_user;
mod util;

use authenticate_user::AuthenticationState;
use nadoeda_models::user::{User, UserId};
pub use teloxide;

use create_daily_reminder::CreatingDailyReminderState;
use dptree::case;
use nadoeda_scheduler::ReminderScheduler;
use nadoeda_storage::{sqlite::{reminder_storage::SqliteReminderStorage, user_storage::SqliteUserInfoStorage}, ReminderStorage};
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue, dispatching::dialogue::InMemStorage, macros::BotCommands, prelude::*,
};

type GlobalDialogue = Dialogue<GlobalState, InMemStorage<GlobalState>>;
type HandlerResult = anyhow::Result<()>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct AuthenticationInfo(User);

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum GlobalState {
    #[default]
    Unauthenticated,
    Authenticating(AuthenticationState),
    AuthenticatedV2(AuthenticationInfo, AuthenticatedActionState),
    Authenticated(AuthenticationInfo),
    CreatingDailyReminder(AuthenticationInfo, CreatingDailyReminderState),
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum AuthenticatedActionState {
    #[default]
    Idle,
    CreatingDailyReminnder(CreatingDailyReminderState)
}

pub struct TelegramInteractionInterface;

impl TelegramInteractionInterface {
    pub async fn start(
        bot: teloxide::Bot,
        scheduler: Arc<dyn ReminderScheduler>,
        reminder_storage: Arc<SqliteReminderStorage>,
        user_storage: Arc<SqliteUserInfoStorage>
    ) {
        log::info!("Starting Telegram UI.");

        let cancel_handler = Update::filter_message().branch(
            teloxide::filter_command::<GlobalCommand, _>()
                .branch(case![GlobalCommand::Cancel].endpoint(cancel)),
        );

        let invalid_state_handler =
            Update::filter_message().branch(dptree::endpoint(invalid_state));

        let invalid_callback_handler =
            Update::filter_callback_query().branch(dptree::endpoint(invalid_query));

        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
            .branch(cancel_handler)
            .branch(authenticate_user::schema())
            .branch(create_daily_reminder::schema())
            .branch(edit_reminders::schema())
            .branch(invalid_state_handler)
            .branch(invalid_callback_handler);

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![
                InMemStorage::<GlobalState>::new(),
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

async fn cancel(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Cancelled current operation.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn invalid_state(bot: Bot, dialogue: GlobalDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Please try again or use /cancel to stop current operation.",
    )
    .await?;
    Ok(())
}

async fn invalid_query(bot: Bot, dialogue: GlobalDialogue, query: CallbackQuery) -> HandlerResult {
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
    CreateReminder,
    ListReminders,
    Cancel,
}
