use crate::reminder::{Reminder, ReminderId};

use super::{
    common::{ReminderManagerMessage, ReminderManagerSender, SchedulerContext},
    scheduler::{ReminderScheduler, ScheduledTask},
    worker::{ReminderWorker, WorkerFactory},
};
use async_trait::async_trait;
use chrono::Duration;
use std::{collections::HashMap, marker::PhantomData};
use tokio::{sync::mpsc, task::JoinHandle};

pub struct ReminderManager {
    channel_sender: ReminderManagerSender,
    manager_task_handle: JoinHandle<()>,
}

#[async_trait]
pub trait ReminderManagerTrait: Send + Sync {
    async fn schedule_reminder(&self, reminder: Reminder) -> anyhow::Result<()>;
    async fn cancel_reminder(&self, reminder: Reminder) -> anyhow::Result<()>;
}

#[async_trait]
impl ReminderManagerTrait for ReminderManager {
    async fn schedule_reminder(&self, reminder: Reminder) -> anyhow::Result<()> {
        self.channel_sender.send_schedule(reminder).await
    }
    
    async fn cancel_reminder(&self, reminder: Reminder) -> anyhow::Result<()> {
        self.channel_sender.send_cancel(reminder).await
    }
}

impl ReminderManager {
    pub fn create(worker_factory: impl WorkerFactory + 'static) -> Self {
        let (channel_sender, receiver) = mpsc::channel(64);
        let sender = ReminderManagerSender::new(channel_sender);
        let tasks_sender = sender.clone();
        let manager_task_handle = tokio::spawn(async move {
            Self::handle_messages(worker_factory, receiver, tasks_sender).await;
        });

        Self {
            channel_sender: sender,
            manager_task_handle,
        }
    }

    async fn handle_messages(
        worker_factory: impl WorkerFactory,
        mut receiver: mpsc::Receiver<ReminderManagerMessage>,
        sender: ReminderManagerSender,
    ) {
        let mut tasks = HashMap::<ReminderId, ScheduledTask>::new();
        while let Some(msg) = receiver.recv().await {
            println!("manager got message! {:?}", msg);
            match msg {
                ReminderManagerMessage::Schedule(reminder) => {
                    let id = reminder.id;
                    if let Some(task) = tasks.remove(&reminder.id) {
                        task.cancel(Duration::seconds(5).to_std().unwrap()).await;
                    }

                    Self::handle_schedule_reminder(
                        &mut tasks,
                        &worker_factory,
                        reminder,
                        sender.clone(),
                    )
                }
                ReminderManagerMessage::ScheduleError(error, reminder) => {
                    let id = reminder.id;
                    tasks.remove(&reminder.id);
                    println!(
                        "Error executing task for reminder. error = {}, reminder_id = {}",
                        error, reminder.id
                    )
                }
                ReminderManagerMessage::ScheduleFinished(reminder) => {
                    tasks.remove(&reminder.id);
                    println!(
                        "Successfully executed worker for reminder. [reminder_id = {}]",
                        reminder.id
                    )
                }
                ReminderManagerMessage::Cancel(reminder) => {
                    let id = reminder.id;
                    if let Some(task) = tasks.remove(&reminder.id) {
                        task.cancel(Duration::seconds(5).to_std().unwrap()).await;
                    }
                }
            }
        }
    }

    fn handle_schedule_reminder(
        tasks: &mut HashMap<ReminderId, ScheduledTask>,
        worker_factory: &impl WorkerFactory,
        reminder: Reminder,
        sender: ReminderManagerSender,
    ) {
        let id = reminder.id;
        let context = SchedulerContext { sender, reminder };
        let worker = worker_factory.create_worker();
        let task = ReminderScheduler::schedule_reminder(context, worker);
        tasks.insert(id, task);
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, NaiveWeek, Timelike, Utc};
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

    use crate::{reminder::{self, Reminder, ReminderFireTime, ReminderId, ReminderState}, scheduling::manager};

    use super::*;

    type ReceivedTasks = Arc<Mutex<Vec<ReminderId>>>;
    struct MockWorkerFactory {
        received_tasks: ReceivedTasks
    }

    struct MockWorker {
        received_tasks: ReceivedTasks
    }

    #[async_trait]
    impl ReminderWorker for MockWorker {
        async fn handle_reminder(&self, context: &SchedulerContext) -> anyhow::Result<()> {
            let mut tasks = self.received_tasks.lock().await;
            tasks.push(context.reminder.id);
            Ok(())
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
    pub async fn basic_scheduling_test() {
        let received_tasks = received_tasks();
        let manager = manager(&received_tasks);
        let reminder = reminder(NaiveTime::from_hms_milli_opt(12, 0, 0, 0).unwrap());
        
        let expected_delay =
            ReminderScheduler::get_target_delay(&reminder.fire_at.time(), Utc::now().naive_utc());
        
        let reminder_id = reminder.id;
        
        manager.schedule_reminder(reminder).await.unwrap();
        tokio::time::sleep(expected_delay.to_std().unwrap() + Duration::from_secs(15)).await;
        let tasks = received_tasks.lock().await;

        assert_eq!(tasks.len(), 1);
        assert_eq!(*tasks.first().unwrap(), reminder_id)
    }

    #[tokio::test(start_paused = true)]
    pub async fn cancelling_test() {
        let received_tasks = received_tasks();
        let manager = manager(&received_tasks);
        let reminder = reminder(NaiveTime::from_hms_milli_opt(12, 00, 00, 00).unwrap());
        let expected_delay = expected_delay(&reminder);

        let reminder_id = reminder.id;
        manager.schedule_reminder(reminder.clone()).await.unwrap();
        manager.cancel_reminder(reminder).await.unwrap();
        tokio::time::sleep(expected_delay.to_std().unwrap() - Duration::from_secs(15)).await;

        wait_for_trigger(expected_delay).await;

        let tasks = received_tasks.lock().await;
        assert_eq!(tasks.len(), 0);
    }

    async fn wait_for_trigger(expected_delay: chrono::Duration) {
        tokio::time::sleep(expected_delay.to_std().unwrap() + Duration::from_secs(15)).await
    } 

    fn expected_delay(reminder: &Reminder) -> chrono::Duration {
        ReminderScheduler::get_target_delay(&reminder.fire_at.time(), Utc::now().naive_utc())
    }

    fn reminder(fire_at: NaiveTime) -> Reminder {
        Reminder {
            id: 1,
            state: ReminderState::Pending,
            fire_at: ReminderFireTime::new(fire_at),
            text: "".to_string(),
        }
    }

    fn manager(received_tasks: &ReceivedTasks) -> ReminderManager {
        let factory = MockWorkerFactory {
            received_tasks: Arc::clone(received_tasks),
        };
        
        ReminderManager::create(factory)
    }
    
    fn received_tasks() -> ReceivedTasks {
        Arc::new(Mutex::new(vec![]))
    }   
}
