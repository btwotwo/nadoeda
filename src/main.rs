#![allow(dead_code, unused_imports, unused_variables)]
mod reminder;
mod scheduling;
mod appsettings;
mod telegram_bot;
use anyhow::ensure;
use appsettings::AppSettings;
use chrono::{DateTime, Days, Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use config::Config;
use scheduling::{ReminderWorker, SchedulerContext, WorkerFactory};
use teloxide::{prelude::Requester, types::Message, Bot};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
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
    let bot = Bot::new(settings.telegram.token.clone());
    
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        log::info!("Chat id {:?}", msg.chat.id);
        bot.send_message(msg.chat.id, "Bruh").await?;
        Ok(())
    }).await;
}
