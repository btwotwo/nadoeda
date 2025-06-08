mod create_reminder;
use crate::appsettings;
use chrono::NaiveTime;
use create_reminder::CreateReminderState;
use dptree::case;
use dptree::prelude::*;
use teloxide::{
    dispatching::dialogue, dispatching::dialogue::InMemStorage, macros::BotCommands, prelude::*,
};

type GlobalDialogue = Dialogue<GlobalState, InMemStorage<GlobalState>>;
type HandlerResult = anyhow::Result<()>;

#[derive(Default, Clone)]
enum GlobalState {
    #[default]
    Idle,
    CreateReminder(CreateReminderState),
}

pub struct TelegramInteractionInterface;
impl TelegramInteractionInterface {
    pub async fn start() {
        let bot = Bot::new(appsettings::get().telegram.token.clone());
        log::info!("Creating Telegram interaction interface");

        let cancel_handler = Update::filter_message().branch(
            teloxide::filter_command::<GlobalCommand, _>()
                .branch(case![GlobalCommand::Cancel].endpoint(cancel)),
        );

        let invalid_state_handler =
            Update::filter_message().branch(dptree::endpoint(invalid_state));

        let schema = dialogue::enter::<Update, InMemStorage<GlobalState>, GlobalState, _>()
            .branch(cancel_handler)
            .branch(create_reminder::schema())
            .branch(invalid_state_handler);

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![InMemStorage::<GlobalState>::new()])
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
    bot.send_message(msg.chat.id, "Unable to handle the message.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum GlobalCommand {
    CreateReminder,
    Cancel,
}
