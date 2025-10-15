mod appsettings;
use std::sync::Arc;

use async_trait::async_trait;
use nadoeda_scheduler::delivery::{ReminderDeliveryChannel, ReminderMessageType};
use nadoeda_delivery_scheduler::DeliveryReminderScheduler;
use nadoeda_models::reminder::Reminder;
use nadoeda_storage::{InMemoryReminderStorage, ReminderStorage};
use nadoeda_telegram::{TelegramInteractionInterface, teloxide};

struct PrinterDeliveryChannel;

#[async_trait]
impl ReminderDeliveryChannel for PrinterDeliveryChannel {
    async fn send_reminder_notification(&self, reminder: &Reminder, message: ReminderMessageType) {
        println!("{:?} - {:?}", reminder, message)
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let storage: Arc<dyn ReminderStorage> = Arc::new(InMemoryReminderStorage::new());
    let scheduler = Arc::new(DeliveryReminderScheduler::new(Arc::new(
        PrinterDeliveryChannel,
    )));
    
    let bot = teloxide::Bot::new(appsettings::get().telegram.token.clone());

    let interface_task = tokio::spawn({
        let storage = storage.clone();
        let scheduler = scheduler.clone();
        let bot = bot.clone();
        async move { TelegramInteractionInterface::start(bot, scheduler, storage).await }
    });

    interface_task.await;
}
