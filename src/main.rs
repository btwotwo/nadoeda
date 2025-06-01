#![allow(dead_code, unused_imports, unused_variables)]
mod common;
mod manager;
mod reminder;
mod scheduler;
mod worker;
use anyhow::ensure;
use chrono::{DateTime, Days, Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use common::{ReminderManagerMessage, ReminderManagerSender, SchedulerContext};
use reminder::{Reminder, ReminderId};
use scheduler::{ReminderScheduler, ScheduledTask};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;
use worker::{ReminderWorker, WorkerFactory};

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
    println!("Hello, world!");
}
