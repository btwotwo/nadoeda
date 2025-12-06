use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use nadoeda_scheduler::delivery::{ReminderDeliveryChannel, ReminderMessageType};
use nadoeda_scheduler::{ReminderScheduler, ScheduleRequest, ScheduledReminder};
use tokio::{
    sync::{RwLock, mpsc, watch},
    task::{self, JoinHandle},
};

use nadoeda_models::reminder::{Reminder, ReminderId, ReminderState};

const NAGGING_ATTEMPTS: u8 = 10;
const NAGGING_TIMEOUT: Duration = Duration::from_secs(30);

const CONFIRMATION_ATTEMPTS: u8 = 10;
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
enum ReminderEvent {
    Schedule,
    Trigger,
    Acknowledge,
    Confirm,
    Stop,
}

struct ScheduledReminderHandle {
    task: JoinHandle<()>,
    tx: mpsc::Sender<ReminderEvent>,
}

struct CleanupTask(watch::Sender<()>);

type ReminderTaskStore = RwLock<HashMap<ReminderId, ScheduledReminderHandle>>;

pub struct DeliveryReminderScheduler {
    tasks: Arc<ReminderTaskStore>,
    delivery_channel: Arc<dyn ReminderDeliveryChannel>,
    cleanup_task: CleanupTask,
}

impl DeliveryReminderScheduler {
    pub fn new(delivery_channel: Arc<dyn ReminderDeliveryChannel>) -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::new()));
        let cleanup_task = Self::spawn_cleanup_task(Arc::clone(&tasks));

        Self {
            tasks,
            delivery_channel,
            cleanup_task,
        }
    }
}

impl Drop for DeliveryReminderScheduler {
    fn drop(&mut self) {
        let _ = self.cleanup_task.0.send(());
    }
}

impl DeliveryReminderScheduler {
    fn create_reminder_task(&self, reminder: Reminder) -> anyhow::Result<ScheduledReminderHandle> {
        let reminder_id = reminder.id;
        log::info!("Starting task for reminder {reminder_id}");
        let (tx, rx) = mpsc::channel(10);

        let tx_clone = tx.clone();
        let delivery_channel = self.delivery_channel.clone();
        let task = task::spawn(async move {
            tx_clone.send(ReminderEvent::Schedule).await.unwrap();
            run_reminder(reminder, delivery_channel.as_ref(), rx, tx_clone).await;
        });

        let scheduled_reminder = ScheduledReminderHandle { task, tx };

        Ok(scheduled_reminder)
    }

    fn spawn_cleanup_task(tasks: Arc<ReminderTaskStore>) -> CleanupTask {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(());
        task::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(300)) => {
                        Self::clean_finished_tasks(&tasks).await;
                    }
                    _ = shutdown_rx.changed() => {
                        log::info!("Cleanup task shutting down");
                        break;
                    }
                };
            }
        });

        CleanupTask(shutdown_tx)
    }

    async fn clean_finished_tasks(tasks: &ReminderTaskStore) {
        let mut tasks = tasks.write().await;
        let before = tasks.len();
        tasks.retain(|_, handle| !handle.task.is_finished());
        let after = tasks.len();

        if before != after {
            log::info!("Cleaned up {} completed reminder tasks", before - after);
        }
    }
}

#[async_trait]
impl ReminderScheduler for DeliveryReminderScheduler {
    async fn schedule_reminder(
        &self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<ScheduledReminder> {
        let reminder_id = schedule_request.reminder.id;
        if let Entry::Vacant(e) = self.tasks.write().await.entry(reminder_id) {
            let scheduled_reminder_handle = self
                .create_reminder_task(schedule_request.reminder)
                .unwrap();

            e.insert(scheduled_reminder_handle);

            Ok(ScheduledReminder { id: reminder_id })
        } else {
            anyhow::bail!("Already scheduled")
        }
    }

    async fn cancel_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        if let Some((_, scheduled_reminder)) = self
            .tasks
            .write()
            .await
            .remove_entry(&scheduled_reminder.id)
        {
            scheduled_reminder.tx.send(ReminderEvent::Stop).await?;

            Ok(())
        } else {
            anyhow::bail!("No such reminder")
        }
    }

    async fn acknowledge_reminder(
        &self,
        scheduled_reminder: &ScheduledReminder,
    ) -> anyhow::Result<()> {
        if let Some(task) = self.tasks.read().await.get(&scheduled_reminder.id) {
            task.tx.send(ReminderEvent::Acknowledge).await?;
        }
        Ok(())
    }

    async fn confirm_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        if let Some(task) = self.tasks.read().await.get(&scheduled_reminder.id) {
            task.tx.send(ReminderEvent::Confirm).await?;
        }

        Ok(())
    }
}

async fn run_reminder(
    mut reminder: Reminder,
    delivery: &dyn ReminderDeliveryChannel,
    mut rx: mpsc::Receiver<ReminderEvent>,
    tx: mpsc::Sender<ReminderEvent>,
) {
    while let Some(event) = rx.recv().await {
        let new_state =
            handle_event(&reminder, &reminder.state, &event, delivery, tx.clone()).await;
        reminder.state = new_state;
        if matches!(event, ReminderEvent::Stop) {
            break;
        }
    }
}

async fn handle_event(
    reminder: &Reminder,
    current_state: &ReminderState,
    event: &ReminderEvent,
    delivery: &dyn ReminderDeliveryChannel,
    tx: mpsc::Sender<ReminderEvent>,
) -> ReminderState {
    // println!("({current_state:?}, {event:?})");
    let id = reminder.id;
    match (current_state, event) {
        (ReminderState::Pending, ReminderEvent::Schedule) => {
            let delay = get_target_delay(reminder.fire_at.time(), Utc::now())
                .to_std()
                .unwrap();

            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Scheduled)
                .await;

            log::info!(
                "[SCHEDULE] Sleeping for {:?} delay. ReminderId {}",
                delay,
                id
            );

            send_after_delay(ReminderEvent::Trigger, tx, delay);

            ReminderState::Scheduled
        }
        (ReminderState::Scheduled, ReminderEvent::Trigger) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Fired)
                .await;

            log::info!(
                "[NAGGING] Sleeping for {:?} delay. ReminderId {}",
                NAGGING_TIMEOUT,
                id
            );

            send_after_delay(ReminderEvent::Trigger, tx, NAGGING_TIMEOUT);

            ReminderState::Nagging {
                attempts_left: NAGGING_ATTEMPTS,
            }
        }
        (ReminderState::Nagging { attempts_left }, ReminderEvent::Trigger) => {
            if *attempts_left == 0 {
                delivery
                    .send_reminder_notification(reminder, ReminderMessageType::Timeout)
                    .await;
                return ReminderState::Pending;
            }

            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Nag)
                .await;

            log::info!(
                "[NAGGING REPEAT] Sleeping for {:?} delay. ReminderId {}",
                NAGGING_TIMEOUT,
                id
            );

            send_after_delay(ReminderEvent::Trigger, tx, NAGGING_TIMEOUT);

            ReminderState::Nagging {
                attempts_left: attempts_left - 1,
            }
        }
        (ReminderState::Nagging { .. }, ReminderEvent::Acknowledge) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Acknowledge)
                .await;

            log::info!(
                "[CONFIRMATION] Sleeping for {:?} delay. ReminderId {}",
                CONFIRMATION_TIMEOUT,
                id
            );

            send_after_delay(ReminderEvent::Trigger, tx, CONFIRMATION_TIMEOUT);

            ReminderState::Confirming {
                attempts_left: CONFIRMATION_ATTEMPTS,
            }
        }
        (ReminderState::Confirming { attempts_left }, ReminderEvent::Trigger) => {
            if *attempts_left == 0 {
                delivery
                    .send_reminder_notification(reminder, ReminderMessageType::Timeout)
                    .await;
                return ReminderState::Pending;
            }

            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Confirmation)
                .await;

            log::info!(
                "[CONFIRMATION REPEAT] Sleeping for {:?} delay. ReminderId {}",
                CONFIRMATION_TIMEOUT,
                id
            );

            send_after_delay(ReminderEvent::Trigger, tx, CONFIRMATION_TIMEOUT);

            ReminderState::Confirming {
                attempts_left: attempts_left - 1,
            }
        }
        (ReminderState::Confirming { .. }, ReminderEvent::Confirm) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Finished)
                .await;
            ReminderState::Pending
        }
        (_, ReminderEvent::Stop) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Stopped)
                .await;
            ReminderState::Pending
        }
        (state, event) => {
            log::warn!(
                "Received unknown state and event combination for reminder. [state = {:?}, event = {:?}, reminder_id = {}]",
                state,
                event,
                reminder.id
            );

            *state
        }
    }
}

fn send_after_delay(ev: ReminderEvent, tx: mpsc::Sender<ReminderEvent>, delay: Duration) {
    task::spawn(async move {
        tokio::time::sleep(delay).await;
        let _ = tx.send(ev).await;
    });
}

pub(crate) fn get_target_delay(fire_at: &NaiveTime, now: DateTime<Utc>) -> chrono::Duration {
    let max_delta = TimeDelta::new(10, 0).expect("This is always in bounds.");
    let delta = *fire_at - now.time();

    let today = now.date_naive();
    let target_date = if delta <= max_delta {
        today
            .checked_add_signed(TimeDelta::days(1))
            .expect("Not realistic to overflow")
    } else {
        today
    };

    let target_datetime = target_date.and_time(*fire_at);

    target_datetime - now.naive_utc()
}

#[cfg(test)]
mod tests;
