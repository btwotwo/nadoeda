#![allow(dead_code, unused_imports, unused_variables)]
mod common;
mod reminder;
mod worker;
use anyhow::ensure;
use chrono::{DateTime, Days, Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use common::{ReminderManagerMessage, ReminderManagerSender, SchedulerContext};
use reminder::{Reminder, ReminderId};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
};
use tokio::{sync::mpsc, task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;
use worker::{ReminderWorker, WorkerFactory};

struct ScheduledTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl ScheduledTask {
    pub async fn cancel(self, timeout: std::time::Duration) {
        self.cancellation_token.cancel();
        let cancel_with_timeout = tokio::time::timeout(timeout, self.task_handle);
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
                        task.cancel(Duration::seconds(10).to_std().unwrap()).await;
                    }

                    Self::handle_schedule_reminder(
                        &mut tasks,
                        &worker_factory,
                        reminder,
                        sender.clone(),
                    )
                }

                ReminderManagerMessage::Cancel(reminder) => {
                    let id = reminder.id;
                    if let Some(task) = tasks.remove(&reminder.id) {
                        task.cancel(Duration::seconds(10).to_std().unwrap()).await;
                    }
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

        let now = Utc::now().naive_utc();
        let delay = Self::get_target_delay(&context.reminder.fire_at, now)
            .to_std()
            .expect("The target delay is always in the future.");

        let task_handle = tokio::spawn(async move {
            Self::handle_reminder_after_delay(task_cancellation_token, context, delay, worker).await
        });

        ScheduledTask {
            task_handle,
            cancellation_token,
        }
    }

    async fn handle_reminder_after_delay<TWorker: ReminderWorker + Send>(
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

    fn get_target_delay(fire_at: &NaiveTime, now: NaiveDateTime) -> Duration {
        let max_delta = TimeDelta::new(10, 0).expect("This is always in bounds.");
        let delta = *fire_at - now.time();

        let today = now.date();
        let target_date = if delta <= max_delta {
            today
                .checked_add_signed(TimeDelta::days(1))
                .expect("Not realistic to overflow")
        } else {
            today
        };

        let target_datetime = target_date.and_time(*fire_at);

        target_datetime - now
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use crate::*;
    use chrono::{Date, Datelike, NaiveDate, NaiveDateTime, NaiveTime, NaiveWeek, Timelike};
    use proptest::prelude::*;
    use proptest_arbitrary_interop::arb;
    use std::{
        any::Any,
        borrow::BorrowMut,
        cell::{Cell, RefCell},
        collections::HashMap,
        io::Read,
        sync::{Arc, RwLock},
        time::Duration,
    };
    use tokio::{sync::Mutex, time};
    use tokio_util::sync::CancellationToken;

    struct MockWorkerFactory {
        received_tasks: Arc<Mutex<Vec<ReminderId>>>,
    }

    struct MockWorker {
        received_tasks: Arc<Mutex<Vec<ReminderId>>>,
    }

    impl ReminderWorker for MockWorker {
        async fn handle_reminder(&self, context: SchedulerContext) {
            let mut tasks = self.received_tasks.lock().await;
            tasks.push(context.reminder.id);
        }
    }

    impl WorkerFactory for MockWorkerFactory {
        type Worker = MockWorker;

        fn create_worker(&self) -> Self::Worker {
            MockWorker {
                received_tasks: self.received_tasks.clone(),
            }
        }
    }

    #[tokio::test(start_paused = true)]
    #[ignore = "need to adjust to the new delay logic"]
    pub async fn basic_scheduling_test() {
        let received_tasks = Arc::new(Mutex::new(vec![]));
        let factory = MockWorkerFactory {
            received_tasks: Arc::clone(&received_tasks),
        };
        let manager = ReminderManager::create(factory);
        let reminder = Reminder {
            id: 1,
            state: reminder::ReminderState::Pending,
            fire_at: chrono::NaiveTime::from_hms_milli_opt(12, 0, 0, 0).unwrap(),
        };
        let reminder_id = reminder.id;
        manager.schedule_reminder(reminder).await.unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
        let tasks = received_tasks.lock().await;

        assert_eq!(tasks.len(), 1);
        assert_eq!(*tasks.first().unwrap(), reminder_id)
    }

    #[test]
    pub fn when_firing_time_is_yet_to_come_target_delay_should_be_less_than_day() {
        let now = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let fire_at = NaiveTime::from_hms_opt(13, 0, 0).unwrap();

        let delay = ReminderScheduler::get_target_delay(&fire_at, now);

        assert_eq!(
            delay.num_hours(),
            1,
            "With given constraints the delay should be 1 hour."
        );
    }

    #[test]
    pub fn when_firing_time_is_passed_target_delay_should_be_next_day() {
        let now = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let fire_at = NaiveTime::from_hms_opt(11, 0, 0).unwrap();
        let delay = ReminderScheduler::get_target_delay(&fire_at, now);

        assert_eq!(
            delay.num_hours(),
            23,
            "With given constraints, the delay should be 23 hours"
        );
    }

    proptest! {
        #[test]
        fn test_target_delay(
            now in arb::<NaiveDateTime>(),
            fire_at in arb::<NaiveTime>()
        ) {
            let fire_at = fire_at.with_nanosecond(0).unwrap();
            let now = now.with_nanosecond(0).unwrap();

            let acceptable_delay = TimeDelta::seconds(0);
            let delay = ReminderScheduler::get_target_delay(&fire_at, now);
            let target_datetime = now + delay;

            assert!(target_datetime > now, "Target time should always be in the future");
            assert!(target_datetime.time() == fire_at, "fire_at = {:?}, target_datetime.time() = {:?}, target_datetime = {:?}", fire_at, target_datetime.time(), target_datetime);
        }
    }
}
