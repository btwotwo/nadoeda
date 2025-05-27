#![allow(dead_code, unused_imports, unused_variables)]
mod reminder;
use anyhow::ensure;
use chrono::{DateTime, Duration, NaiveTime, Utc};
use reminder::{Reminder, ReminderId};
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
    pub async fn cancel(self) {
        self.cancellation_token.cancel();
        //todo: handle error
        let _ = self.task_handle.await;
    }
}

trait WorkerFactory<TParams> {
    type Worker: ReminderWorker;

    fn create_worker(&self, worker_creation_params: TParams) -> Self::Worker;
}

type PrinterWorkerParams = ();
struct PrinterWorkerFactory;
impl WorkerFactory<PrinterWorkerParams> for PrinterWorkerFactory {
    type Worker = PrinterWorker;

    fn create_worker(&self, worker_creation_params: PrinterWorkerParams) -> Self::Worker {
        todo!()
    }
}

trait ReminderWorker {
    fn handle_reminder(self, context: ReminderSchedulerContext) -> impl Future<Output = ()> + Send;
}

struct PrinterWorker;
impl ReminderWorker for PrinterWorker {
    async fn handle_reminder(self, ctx: ReminderSchedulerContext) {
        eprintln!("Firing reminder {:?}!", ctx.reminder);
    }
}

#[derive(Debug)]
enum ReminderManagerMessage {
    Schedule(Reminder),
}

struct ReminderManager<TFactory = PrinterWorkerFactory, TFactoryParams = PrinterWorkerParams>
where
    TFactory: WorkerFactory<TFactoryParams>,
{
    sender: ReminderManagerSender,
    manager_task_handle: JoinHandle<()>,
    worker_factory: TFactory,
    factory_params: TFactoryParams
}

type ReminderManagerSender = mpsc::Sender<ReminderManagerMessage>;

impl<TFactory, TFactoryParams> ReminderManager<TFactory, TFactoryParams>
where
    TFactory: WorkerFactory<TFactoryParams>,
    TFactory::Worker: Send + Sync,
    TFactoryParams: Clone
{
    pub fn create(worker_factory: TFactory, factory_params: TFactoryParams) -> Self {
        let (sender, receiver) = mpsc::channel(64);
        let tasks_sender = sender.clone();
        let manager_task_handle = tokio::spawn(async move {
            Self::handle_messages(receiver, tasks_sender).await;
        });

        Self {
            sender,
            manager_task_handle,
            worker_factory,
            factory_params
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
        mut receiver: mpsc::Receiver<ReminderManagerMessage>,
        sender: mpsc::Sender<ReminderManagerMessage>,
    ) {
        let mut state = HashMap::<ReminderId, ScheduledTask>::new();
        while let Some(msg) = receiver.recv().await {
            println!("manager got message! {:?}", msg);
            match msg {
                ReminderManagerMessage::Schedule(reminder) => {
                    Self::handle_schedule_reminder(&mut state, reminder, sender.clone()).await
                }
            }
        }
    }

    async fn handle_schedule_reminder(
        tasks: &mut HashMap<ReminderId, ScheduledTask>,
        reminder: Reminder,
        sender: ReminderManagerSender,
    ) {
        let id = reminder.id;
        if let Some(task) = tasks.remove(&reminder.id) {
            //todo add timeout
            task.cancel().await;
        }

        let context = ReminderSchedulerContext { reminder, sender };

        let worker = self.get_worker();
        let task = ReminderScheduler::schedule_reminder(context, worker);
        tasks.insert(id, task);
    }

    fn get_worker(&self) -> TFactory::Worker {
        self.worker_factory.create_worker(self.factory_params.clone())
    }
}

struct ReminderScheduler {}

struct ReminderSchedulerContext {
    sender: ReminderManagerSender,
    reminder: Reminder,
}

impl ReminderScheduler {
    pub fn schedule_reminder<TWorker: ReminderWorker + Send + Sync + 'static>(
        context: ReminderSchedulerContext,
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

    async fn do_work<TWorker: ReminderWorker + Send + Sync>(
        cancellation_token: CancellationToken,
        ctx: ReminderSchedulerContext,
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

        manager.schedule_reminder(reminder).await;
        tokio::time::sleep(Duration::from_secs(11)).await;
    }
}
