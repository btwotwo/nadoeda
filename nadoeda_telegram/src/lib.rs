mod create_daily_reminder;
mod edit_reminders;

pub use teloxide;

use create_daily_reminder::CreatingDailyReminderState;
use dptree::case;
use nadoeda_scheduler::ReminderScheduler;
use nadoeda_storage::ReminderStorage;
use std::sync::Arc;
use teloxide::{
    dispatching::dialogue, dispatching::dialogue::InMemStorage, macros::BotCommands, prelude::*,
};

type GlobalDialogue = Dialogue<GlobalState, InMemStorage<GlobalState>>;
type HandlerResult = anyhow::Result<()>;
type HandlerReminderStorageType = Arc<dyn ReminderStorage>;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum GlobalState {
    #[default]
    Idle,
    CreatingDailyReminder(CreatingDailyReminderState),
}

pub struct TelegramInteractionInterface;
impl TelegramInteractionInterface {
    pub async fn start(
        bot: teloxide::Bot,
        scheduler: Arc<dyn ReminderScheduler>,
        reminder_storage: Arc<dyn ReminderStorage>,
    ) {
        log::info!("Starting Telegram interaction interface");

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
            .branch(create_daily_reminder::schema())
            .branch(edit_reminders::schema())
            .branch(invalid_state_handler)
            .branch(invalid_callback_handler);

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![
                InMemStorage::<GlobalState>::new(),
                scheduler,
                reminder_storage
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
