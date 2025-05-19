#![allow(dead_code, unused_imports, unused_variables)]

use anyhow::ensure;
use chrono::{DateTime, Duration, Utc};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
};
use tokio::{task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
enum ReminderState {
    Scheduled,
    Nagging,
    Completed,
}

#[derive(Debug)]
struct Reminder {
    id: u64,
    state: ReminderState,
    fire_at: chrono::NaiveTime,
}

struct ScheduledTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

trait ReminderWorker {
    fn handle_reminder(&self, reminder: Reminder) -> impl Future<Output = ()> + Send;
}

struct PrinterWorker;
impl ReminderWorker for PrinterWorker {
    async fn handle_reminder(&self, reminder: Reminder) {
        eprintln!("Firing reminder {:?}!", reminder);
    }
}


struct ReminderManager {
    tasks: HashMap<i32, ScheduledTask>
}


struct ReminderScheduler {}

impl ReminderScheduler {
    pub fn schedule_reminder<TWorker: ReminderWorker + Send + Sync + 'static>(
        &mut self,
        reminder: Reminder,
        worker: TWorker,
    ) {
        let cancellation_token = CancellationToken::new();
        let task_cancellation_token = cancellation_token.child_token();

        let reminder_id = reminder.id;

        let now = Utc::now();
        let delay = Self::get_target_delay(&reminder, now)
            .to_std()
            .expect("The target delay is always in the future.");

        let task_handle = tokio::spawn(async move {
            Self::do_work(task_cancellation_token, reminder, delay, worker).await
        });

        ScheduledTask {
            task_handle,
            cancellation_token,
        };
    }

    fn get_target_delay(reminder: &Reminder, now: DateTime<Utc>) -> Duration {
        Duration::seconds(10)
    }

    async fn do_work<TWorker: ReminderWorker + Send + Sync>(
        cancellation_token: CancellationToken,
        reminder: Reminder,
        delay: std::time::Duration,
        worker: TWorker,
    ) {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                println!("Task for scheduling reminder was cancelled. {:?}", reminder)
            },
            _ = tokio::time::sleep(delay) => {
                worker.handle_reminder(reminder).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use crate::*;
    use chrono::NaiveTime;
    use std::{
        borrow::BorrowMut,
        cell::{Cell, RefCell},
        collections::HashMap,
        time::Duration,
    };
    use tokio::time;
    use tokio_util::sync::CancellationToken;
}
