#![allow(dead_code, unused_imports, unused_variables)]
mod appsettings;
mod reminder;
mod scheduling;
mod storage;
mod telegram;
mod telegram_bot;

use anyhow::ensure;
use appsettings::AppSettings;
use async_trait::async_trait;
use chrono::{DateTime, Days, Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use config::Config;
use reminder::Reminder;
use scheduling::{ReminderWorker, SchedulerContext, WorkerFactory};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
};
use storage::{InMemoryReminderStorage, ReminderStorage};
use telegram::TelegramInteractionInterface;
use telegram_bot::TelegramDeliveryChannel;
use teloxide::{
    Bot,
    prelude::Requester,
    types::{ChatId, Message},
};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;

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
