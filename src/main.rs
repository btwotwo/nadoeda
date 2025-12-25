mod appsettings;

use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use nadoeda_delivery_scheduler::DeliveryReminderScheduler;
use nadoeda_models::reminder::Reminder;
use nadoeda_scheduler::delivery::{ReminderDeliveryChannel, ReminderMessageType};
use nadoeda_storage::sqlite::{
    reminder_storage::SqliteReminderStorage, sqlx::SqlitePool, user_storage::SqliteUserInfoStorage,
};
use nadoeda_telegram::delivery::TelegramDeliveryChannel;
use nadoeda_telegram::{teloxide};
use nadoeda_telegram::ui::TelegramInteractionInterface;

struct PrinterDeliveryChannel;

#[async_trait]
impl ReminderDeliveryChannel for PrinterDeliveryChannel {
    async fn send_reminder_notification(
        &self,
        reminder: &Reminder,
        message: ReminderMessageType,
    ) -> Result<(), Box<dyn Error>> {
        println!("{:?} - {:?}", reminder, message);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let sqlite_pool = SqlitePool::connect("sqlite:///tmp/nadoeda.db")
        .await
        .expect("Error creating SQLite pool");
    let storage: Arc<SqliteReminderStorage> =
        Arc::new(SqliteReminderStorage::new(sqlite_pool.clone()));
    let user_storage: Arc<SqliteUserInfoStorage> =
        Arc::new(SqliteUserInfoStorage::new(sqlite_pool.clone()));

    let bot = teloxide::Bot::new(appsettings::get().telegram.token.clone());
    let tg_delivery: Arc<dyn ReminderDeliveryChannel> = Arc::new(TelegramDeliveryChannel::new(
        Arc::clone(&user_storage),
        bot.clone(),
    ));

    let scheduler = Arc::new(DeliveryReminderScheduler::new(Arc::clone(&tg_delivery)));

    let interface_task = tokio::spawn({
        let storage = storage.clone();
        let user_storage = user_storage.clone();
        let scheduler = scheduler.clone();
        let bot = bot.clone();
        async move { TelegramInteractionInterface::start(bot, scheduler, storage, user_storage).await }
    });

    interface_task.await.expect("Error in the interface task");
}
