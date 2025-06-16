#![allow(dead_code, unused_imports, unused_variables)]
mod appsettings;
mod reminder;
mod scheduling;
mod telegram;
mod telegram_bot;
mod storage;

use anyhow::ensure;
use appsettings::AppSettings;
use chrono::{DateTime, Days, Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use config::Config;
use scheduling::{ReminderWorker, SchedulerContext, WorkerFactory};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
};
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

    let bot = TelegramDeliveryChannel::create();
    let interface_task = tokio::spawn(async move {
        TelegramInteractionInterface::start().await;
    });

    bot.send_message("Restarted", ChatId(185992715)).await;
    interface_task.await;
}
