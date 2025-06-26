mod appsettings;
mod reminder;
mod scheduling;
mod storage;
mod telegram;
mod telegram_bot;

use async_trait::async_trait;
use scheduling::{ReminderWorker, SchedulerContext, WorkerFactory};
use std::sync::Arc;
use storage::{InMemoryReminderStorage, ReminderStorage};
use telegram::TelegramInteractionInterface;
use telegram_bot::TelegramDeliveryChannel;
use teloxide::{prelude::Requester, types::ChatId};

struct PrinterWorker;
struct PrinterWorkerFactory;
impl WorkerFactory for PrinterWorkerFactory {
    type Worker = PrinterWorker;

    fn create_worker(&self) -> Self::Worker {
        PrinterWorker
    }
}

#[async_trait]
impl ReminderWorker for PrinterWorker {
    async fn handle_reminder(&self, ctx: &SchedulerContext) -> anyhow::Result<()> {
        println!("Firing reminder {:?}!", ctx.reminder);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let settings = appsettings::get();
    let storage: Arc<dyn ReminderStorage + Send + Sync> = Arc::new(InMemoryReminderStorage::new());
    let bot = TelegramDeliveryChannel::create();
    let interface_task = tokio::spawn(async move {
        TelegramInteractionInterface::start(storage.clone()).await;
    });

    bot.send_message("Restarted", ChatId(185992715)).await;

    interface_task.await;
}
