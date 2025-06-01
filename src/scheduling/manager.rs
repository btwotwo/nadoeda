use crate::reminder::{Reminder, ReminderId};

use super::{
    common::{ReminderManagerMessage, ReminderManagerSender, SchedulerContext},
    scheduler::{ReminderScheduler, ScheduledTask},
    worker::{ReminderWorker, WorkerFactory},
};
use chrono::Duration;
use std::{collections::HashMap, marker::PhantomData};
use tokio::{sync::mpsc, task::JoinHandle};

pub struct ReminderManager<TFactory: WorkerFactory> {
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
        let (channel_sender, receiver) = mpsc::channel(64);
        let sender = ReminderManagerSender::new(channel_sender);
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

    pub async fn schedule_reminder(&self, reminder: Reminder) -> anyhow::Result<()> {
        self.sender.schedule(reminder).await
    }

    async fn handle_messages(
        worker_factory: TFactory,
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
        worker_factory: &TFactory,
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

    use crate::{reminder::{Reminder, ReminderFireTime, ReminderId, ReminderState}, scheduling::{scheduler::ReminderScheduler, ReminderManager, ReminderWorker, SchedulerContext, WorkerFactory}};

    struct MockWorkerFactory {
        received_tasks: Arc<Mutex<Vec<ReminderId>>>,
    }

    struct MockWorker {
        received_tasks: Arc<Mutex<Vec<ReminderId>>>,
    }

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
        let received_tasks = Arc::new(Mutex::new(vec![]));
        let factory = MockWorkerFactory {
            received_tasks: Arc::clone(&received_tasks),
        };
        let manager = ReminderManager::create(factory);
        let reminder = Reminder {
            id: 1,
            state: ReminderState::Pending,
            fire_at: ReminderFireTime::new(NaiveTime::from_hms_milli_opt(12, 0, 0, 0).unwrap()),
        };
        let expected_delay =
            ReminderScheduler::get_target_delay(&reminder.fire_at.time(), Utc::now().naive_utc());
        let reminder_id = reminder.id;
        manager.schedule_reminder(reminder).await.unwrap();
        tokio::time::sleep(expected_delay.to_std().unwrap() + Duration::from_secs(15)).await;
        let tasks = received_tasks.lock().await;

        assert_eq!(tasks.len(), 1);
        assert_eq!(*tasks.first().unwrap(), reminder_id)
    }
}
