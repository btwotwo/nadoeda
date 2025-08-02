mod create_daily_reminder;
mod edit_reminders;
mod delivery_channel;

use std::sync::Arc;
pub use delivery_channel::TelegramDeliveryChannel;
use crate::storage::InMemoryReminderStorage;
use crate::PrinterWorkerFactory;
use crate::appsettings;
use crate::scheduling::ReminderManager;
use crate::scheduling::ReminderManagerTrait;
use crate::storage::ReminderStorage;
use create_daily_reminder::CreatingDailyReminderState;
use dptree::case;
use teloxide::{
    dispatching::dialogue, dispatching::dialogue::InMemStorage, macros::BotCommands, prelude::*,
};

type GlobalDialogue = Dialogue<GlobalState, InMemStorage<GlobalState>>;
type HandlerResult = anyhow::Result<()>;
type HandlerReminderStorageType = Arc<dyn ReminderStorage>;

#[derive(Default, Clone)]
enum GlobalState {
    #[default]
    Idle,
    CreatingDailyReminder(CreatingDailyReminderState),
}

pub struct TelegramInteractionInterface;
impl TelegramInteractionInterface {
    pub async fn start(storage: HandlerReminderStorageType) {
        let bot = Bot::new(appsettings::get().telegram.token.clone());
        log::info!("Starting Telegram interaction interface");

        let cancel_handler = Update::filter_message().branch(
            teloxide::filter_command::<GlobalCommand, _>()
                .branch(case![GlobalCommand::Cancel].endpoint(cancel)),
        );

        let invalid_state_handler =
            Update::filter_message().branch(dptree::endpoint(invalid_state));
        let worker_factory = PrinterWorkerFactory;
        
        let manager = ReminderManager::create(worker_factory);
        let manager: Arc<dyn ReminderManagerTrait> = Arc::new(manager);

        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
            .branch(cancel_handler)
            .branch(create_daily_reminder::schema())
            .branch(edit_reminders::schema())
            .branch(invalid_state_handler);

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![
                InMemStorage::<GlobalState>::new(),
                storage,
                manager
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
