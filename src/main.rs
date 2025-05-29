#![allow(dead_code, unused_imports, unused_variables)]
mod reminder;
mod worker;
mod common;
use anyhow::ensure;
use chrono::{DateTime, Duration, NaiveTime, Utc};
use common::{ReminderManagerMessage, ReminderManagerSender, SchedulerContext};
use reminder::{Reminder, ReminderId};
use worker::{ReminderWorker, WorkerFactory};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;

struct ScheduledTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl ScheduledTask {
    pub async fn cancel(self, timeout: Duration) {
        self.cancellation_token.cancel();
        let cancel_with_timeout = tokio::time::timeout(timeout.to_std().unwrap(), self.task_handle);
        let _ = cancel_with_timeout.await;
    }
}

struct PrinterWorker;
struct PrinterWorkerFactory;
impl WorkerFactory for PrinterWorkerFactory {
    type Worker = PrinterWorker;

    fn create_worker(&self) -> Self::Worker {
        PrinterWorker
    }
}

impl ReminderWorker for PrinterWorker {
    async fn handle_reminder(&self, ctx: SchedulerContext) {
        eprintln!("Firing reminder {:?}!", ctx.reminder);
    }
}


struct ReminderManager<TFactory = PrinterWorkerFactory>
where
    TFactory: WorkerFactory,
{
    sender: ReminderManagerSender,
    manager_task_handle: JoinHandle<()>,
    _marker: PhantomData<TFactory>,
}

impl<TFactory> ReminderManager<TFactory>
where
    TFactory: WorkerFactory + Send + 'static,
    TFactory::Worker: ReminderWorker + Send,
{
    pub fn create(worker_factory: TFactory) -> Self {
        let (sender, receiver) = mpsc::channel(64);
        let tasks_sender = sender.clone();
        let manager_task_handle = tokio::spawn(async move {
            Self::handle_messages(worker_factory, receiver, tasks_sender).await;
        });

        Self {
            sender,
            manager_task_handle,
            _marker: PhantomData,
        }
    }

    pub async fn schedule_reminder(
        &self,
        reminder: Reminder,
    ) -> Result<(), mpsc::error::SendError<ReminderManagerMessage>> {
        self.sender
            .send(ReminderManagerMessage::Schedule(reminder))
            .await
    }

    async fn handle_messages(
        worker_factory: TFactory,
        mut receiver: mpsc::Receiver<ReminderManagerMessage>,
        sender: mpsc::Sender<ReminderManagerMessage>,
    ) {
        let mut tasks = HashMap::<ReminderId, ScheduledTask>::new();
        while let Some(msg) = receiver.recv().await {
            println!("manager got message! {:?}", msg);
            match msg {
                ReminderManagerMessage::Schedule(reminder) => {
                    let id = reminder.id;
                    if let Some(task) = tasks.remove(&reminder.id) {
                        task.cancel(Duration::seconds(10)).await;
                    }

                    Self::handle_schedule_reminder(
                        &mut tasks,
                        &worker_factory,
                        reminder,
                        sender.clone(),
                    )
                }
            }
        }
    }

    fn handle_schedule_reminder(
        tasks: &mut HashMap<ReminderId, ScheduledTask>,
        worker_factory: &TFactory,
        reminder: Reminder,
        sender: ReminderManagerSender,
    ) {
        let id = reminder.id;
        let context = SchedulerContext { reminder, sender };
        let worker = worker_factory.create_worker();
        let task = ReminderScheduler::schedule_reminder(context, worker);
        tasks.insert(id, task);
    }
}

struct ReminderScheduler {}


impl ReminderScheduler {
    pub fn schedule_reminder<TWorker: ReminderWorker + Send + 'static>(
        context: SchedulerContext,
        worker: TWorker,
    ) -> ScheduledTask {
        let cancellation_token = CancellationToken::new();
        let task_cancellation_token = cancellation_token.child_token();

        let reminder_id = context.reminder.id;

        let now = Utc::now();
        let delay = Self::get_target_delay(&context.reminder, now)
            .to_std()
            .expect("The target delay is always in the future.");

        let task_handle = tokio::spawn(async move {
            Self::do_work(task_cancellation_token, context, delay, worker).await
        });

        ScheduledTask {
            task_handle,
            cancellation_token,
        }
    }

    fn get_target_delay(reminder: &Reminder, now: DateTime<Utc>) -> Duration {
        Duration::seconds(10)
    }

    async fn do_work<TWorker: ReminderWorker + Send>(
        cancellation_token: CancellationToken,
        ctx: SchedulerContext,
        delay: std::time::Duration,
        worker: TWorker,
    ) {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                println!("Task for scheduling reminder was cancelled. {:?}", ctx.reminder)
            },
            _ = tokio::time::sleep(delay) => {
                worker.handle_reminder(ctx).await;
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

    #[tokio::test(start_paused = true)]
    pub async fn integration() {
        let manager = ReminderManager::create(PrinterWorkerFactory);
        let reminder = Reminder {
            id: 1,
            state: reminder::ReminderState::Pending,
            fire_at: chrono::NaiveTime::from_hms_milli_opt(12, 0, 0, 0).unwrap(),
        };

        manager.schedule_reminder(reminder).await.unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
    }
}
