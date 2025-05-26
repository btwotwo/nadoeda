#![allow(dead_code, unused_imports, unused_variables)]

use anyhow::ensure;
use chrono::{DateTime, Duration, NaiveTime, Utc};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
enum ReminderState {
    Pending,
    Scheduled,
    Nagging,
    Completed,
}

type ReminderId = u64;
#[derive(Debug)]
struct Reminder {
    id: ReminderId,
    state: ReminderState,
    fire_at: chrono::NaiveTime,
}

struct ScheduledTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl ScheduledTask {
    pub async fn cancel(self) {
        self.cancellation_token.cancel();
        self.task_handle.await;
    }
}

trait ReminderWorker {
    fn handle_reminder(self, reminder: Reminder) -> impl Future<Output = ()> + Send;
}

struct PrinterWorker;
impl ReminderWorker for PrinterWorker {
    async fn handle_reminder(self, reminder: Reminder) {
        eprintln!("Firing reminder {:?}!", reminder);
    }
}

#[derive(Debug)]
enum ReminderManagerMessage {
    Schedule(Reminder),
}
struct ReminderManager {
    tx: mpsc::Sender<ReminderManagerMessage>,
    manager_task_handle: JoinHandle<()>,
}

impl ReminderManager {
    pub fn create(reminders: Vec<Reminder>) -> ReminderManager {
        let (sender, receiver) = mpsc::channel(64);
        let task = tokio::spawn(async move {
            Self::handle_messages(receiver).await;
        });

        Self {
            tx: sender,
            manager_task_handle: task,
        }
    }

    async fn handle_messages(mut receiver: mpsc::Receiver<ReminderManagerMessage>) {
        let mut state = HashMap::<ReminderId, ScheduledTask>::new();
        while let Some(cmd) = receiver.recv().await {
            println!("manager got command! {:?}", cmd);
            match cmd {
                ReminderManagerMessage::Schedule(reminder) => Self::schedule_reminder(&mut state, reminder).await
            }
        }
    }

    async fn schedule_reminder(tasks: &mut HashMap<ReminderId, ScheduledTask>, reminder: Reminder) {
        let id = reminder.id;
        if let Some(task) = tasks.remove(&reminder.id) {
            //todo add timeout
            task.cancel().await;
        }

        let worker = Self::get_worker();
        let task = ReminderScheduler::schedule_reminder(reminder, worker);
        tasks.insert(id, task);
    }

    fn get_worker() -> PrinterWorker {
        PrinterWorker
    }
}


struct ReminderScheduler {}

impl ReminderScheduler {
    pub fn schedule_reminder<TWorker: ReminderWorker + Send + Sync + 'static>(
        reminder: Reminder,
        worker: TWorker,
    ) -> ScheduledTask {
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
        }
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
