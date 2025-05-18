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

struct ReminderScheduler {
    tasks: HashMap<u64, ScheduledTask>,
    cancellation_token: CancellationToken,
}

impl ReminderScheduler {
    pub fn schedule_reminder<TWorker: ReminderWorker + Send + Sync>(
        &mut self,
        reminder: Reminder,
        worker: &'static TWorker,
    ) {
        let cancellation_token = self.cancellation_token.child_token();
        let task_cancellation_token = cancellation_token.clone();

        let reminder_id = reminder.id;

        let previous_task = self.tasks.remove(&reminder.id);

        let now = Utc::now();
        let delay = Self::get_target_delay(&reminder, now)
            .to_std()
            .expect("The target delay is always in the future.");

        let task_handle = tokio::spawn(async move {
            Self::do_work(
                previous_task,
                task_cancellation_token,
                reminder,
                delay,
                worker,
            )
            .await
        });

        let scheduled_task = ScheduledTask {
            task_handle,
            cancellation_token,
        };

        self.tasks.insert(reminder_id, scheduled_task);
    }

    fn get_target_delay(reminder: &Reminder, now: DateTime<Utc>) -> Duration {
        Duration::seconds(10)
    }

    async fn do_work<TWorker: ReminderWorker + Send + Sync>(
        previous_task: Option<ScheduledTask>,
        cancellation_token: CancellationToken,
        reminder: Reminder,
        delay: std::time::Duration,
        worker: &TWorker,
    ) {
        if let Some(previous_task) = previous_task {
            Self::cancel_existing(previous_task).await;
        }

        tokio::select! {
            _ = cancellation_token.cancelled() => {
                println!("Task for scheduling reminder was cancelled. {:?}", reminder)
            },
            _ = tokio::time::sleep(delay) => {
                worker.handle_reminder(reminder).await;
            }
        }
    }

    async fn cancel_existing(schedule_task: ScheduledTask) {
        schedule_task.cancellation_token.cancel();
        schedule_task
            .task_handle
            .await
            .unwrap_or_else(|e| println!("Error while cancelling task. {:?}", e))
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
    use tokio::time;
    use std::{borrow::BorrowMut, cell::{Cell, RefCell}, collections::HashMap, time::Duration};
    use tokio_util::sync::CancellationToken;

    unsafe impl Send for MockWorker {}
    unsafe impl Sync for MockWorker {}
    struct MockWorker {
        received_reminders: RefCell<Vec<Reminder>>
    }
    
    impl ReminderWorker for MockWorker {
        async fn handle_reminder(&self, reminder: Reminder) {
            let mut a = self.received_reminders.borrow_mut();
            a.push(reminder);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn given_scheduler_when_reminder_scheduled_should_be_processed_after_timeout() {
        let mut scheduler = ReminderScheduler {
            cancellation_token: CancellationToken::new(),
            tasks: HashMap::new(),
        };

        let reminder = Reminder {
            id: 0,
            state: ReminderState::Scheduled,
            fire_at: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        };
        
        let worker = Box::leak(Box::new(MockWorker {
            received_reminders: RefCell::new(Vec::new())
        }));

        scheduler.schedule_reminder(reminder, worker);
        time::sleep(Duration::from_secs(11)).await;
        assert!(worker.received_reminders.borrow().len() == 1)
    }
}
