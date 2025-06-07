use chrono::NaiveTime;
use dptree::case;
use teloxide::{
    dispatching::dialogue, dispatching::dialogue::InMemStorage, macros::BotCommands, prelude::*,
};

use crate::appsettings;

pub struct TelegramDeliveryChannel {
    bot: Bot,
}

impl TelegramDeliveryChannel {
    pub fn create() -> Self {
        let bot = Bot::new(appsettings::get().telegram.token.clone());

        Self { bot }
    }

    pub async fn send_message(&self, msg: &str, chat_id: ChatId) -> anyhow::Result<()> {
        self.bot.send_message(chat_id, msg).await?;
        Ok(())
    }
}

pub struct TelegramInteractionInterface;

#[derive(Clone, Default)]
enum CreateReminderDialogueState {
    #[default]
    Start,
    ReceiveText,
    ReceiveFiringTime {
        text: String,
    },
    ReceiveFiringPeriod {
        text: String,
        firing_time: String,
    },
}

type ReminderCreationDialogue =
    Dialogue<CreateReminderDialogueState, InMemStorage<CreateReminderDialogueState>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

impl TelegramInteractionInterface {
    pub async fn start() {
        let bot = Bot::new(appsettings::get().telegram.token.clone());
        log::info!("Creating Telegram interaction interface");

        let command_handler = teloxide::filter_command::<Command, _>()
            .branch(
                case![CreateReminderDialogueState::Start]
                    .branch(case![Command::CreateReminder].endpoint(Self::create_reminder_start)),
                
            )
            .branch(case![Command::Cancel].endpoint(Self::cancel));

        let message_handler = Update::filter_message()
            .branch(command_handler)
            .branch(
                case![CreateReminderDialogueState::ReceiveText]
                    .endpoint(Self::receive_reminder_text),
            )
            .branch(case![CreateReminderDialogueState::ReceiveFiringTime {text}].endpoint(Self::receive_firing_time))
            .branch(dptree::endpoint(Self::invalid_state));

        let schema = dialogue::enter::<
            Update,
            InMemStorage<CreateReminderDialogueState>,
            CreateReminderDialogueState,
            _,
        >()
        .branch(message_handler);

        Dispatcher::builder(bot, schema)
            .dependencies(dptree::deps![
                InMemStorage::<CreateReminderDialogueState>::new()
            ])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await
    }

    async fn create_reminder_start(
        bot: Bot,
        dialogue: ReminderCreationDialogue,
        msg: Message,
    ) -> HandlerResult {
        bot.send_message(
            msg.chat.id,
            "Creating a new reminder! Please input reminder text. If you want to cancel, use the /cancel command.",
        )
        .await?;
        dialogue
            .update(CreateReminderDialogueState::ReceiveText)
            .await?;

        Ok(())
    }

    async fn receive_reminder_text(
        bot: Bot,
        dialogue: ReminderCreationDialogue,
        msg: Message,
    ) -> HandlerResult {
        match msg.text() {
            Some(text) => {
                bot.send_message(
                    msg.chat.id,
                    format!("Great! Your reminder will say \"{}\"", text),
                )
                .await?;
                dialogue
                    .update(CreateReminderDialogueState::ReceiveFiringTime {
                        text: text.to_string(),
                    })
                    .await?;
            }
            None => {
                bot.send_message(msg.chat.id, "Please send me reminder text.")
                    .await?;
            }
        }

        Ok(())
    }

    async fn receive_firing_time(
        bot: Bot,
        dialogue: ReminderCreationDialogue,
        text: String,
        msg: Message
    ) -> HandlerResult {
        match msg.text().map(|text| NaiveTime::parse_from_str(text, "%H:%M")) {
            Some(Ok(time)) => {
                bot.send_message(msg.chat.id, format!("Great! Your reminder will fire at {}", time)).await?;
            }
            _ => {
                bot.send_message(msg.chat.id, "Could not parse time. Please send time in the following format: \"13:00\"").await?;
            }
        }
        Ok(())
    }

    async fn invalid_state(
        bot: Bot,
        dialogue: ReminderCreationDialogue,
        msg: Message,
    ) -> HandlerResult {
        bot.send_message(msg.chat.id, "Unable to handle the message.")
            .await?;
        Ok(())
    }

    async fn cancel(bot: Bot, dialogue: ReminderCreationDialogue, msg: Message) -> HandlerResult {
        bot.send_message(msg.chat.id, "Cancelled current operation.")
            .await?;
        dialogue.exit().await?;
        Ok(())
    }
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    CreateReminder,
    Cancel,
}
