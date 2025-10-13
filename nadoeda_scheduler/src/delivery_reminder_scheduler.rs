pub mod delivery;

use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use tokio::{
    sync::{mpsc, RwLock},
    task::{self, JoinHandle},
};

use nadoeda_models::reminder::{Reminder, ReminderId, ReminderState};

use crate::{ReminderDeliveryChannel, ReminderMessageType, scheduler::ScheduleRequest};

use super::ReminderScheduler;

const NAGGING_ATTEMPTS: u8 = 10;
const NAGGING_TIMEOUT: Duration = Duration::from_secs(30);

const CONFIRMATION_ATTEMPTS: u8 = 10;
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

struct ScheduledReminderHandle {
    task: JoinHandle<()>,
    tx: mpsc::Sender<ReminderEvent>,
}

pub struct DeliveryReminderScheduler {
    tasks: RwLock<HashMap<ReminderId, ScheduledReminderHandle>>,
    delivery_channel: Arc<dyn ReminderDeliveryChannel>,
}

impl DeliveryReminderScheduler {
    pub fn new(delivery_channel: Arc<dyn ReminderDeliveryChannel>) -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            delivery_channel,
        }
    }
}

#[async_trait]
impl ReminderScheduler for DeliveryReminderScheduler {
    async fn schedule_reminder(
        &self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<super::ScheduledReminder> {
        let reminder_id = schedule_request.reminder.id;
        if let Entry::Vacant(e) = self.tasks.write().await.entry(reminder_id) {
            let (tx, rx) = mpsc::channel(10);
            let tx_clone = tx.clone();
            let delivery_channel = self.delivery_channel.clone();
            let task = task::spawn(async move {
                tx_clone.send(ReminderEvent::Schedule).await.unwrap();
                run_reminder(
                    schedule_request.reminder,
                    delivery_channel.as_ref(),
                    rx,
                    tx_clone,
                )
                .await;
            });
            let scheduled_reminder = ScheduledReminderHandle { task, tx };
            e.insert(scheduled_reminder);
            Ok(super::ScheduledReminder { id: reminder_id })
        } else {
            anyhow::bail!("Already scheduled")
        }
    }

    async fn cancel_reminder(
        &self,
        scheduled_reminder: &super::ScheduledReminder,
    ) -> anyhow::Result<()> {
        if let Some((_, scheduled_reminder)) = self.tasks.write().await.remove_entry(&scheduled_reminder.id) {
            scheduled_reminder
                .tx
                .send(ReminderEvent::Stop)
                .await?;

            Ok(())
        } else {
            anyhow::bail!("No such reminder")
        }
    }

    async fn acknowledge_reminder(
        &self,
        scheduled_reminder: &super::ScheduledReminder,
    ) -> anyhow::Result<()> {
        if let Some(task) = self.tasks.read().await.get(&scheduled_reminder.id) {
            task.tx.send(ReminderEvent::Acknowledge).await?;
        }
        Ok(())
    }

    async fn confirm_reminder(
        &self,
        scheduled_reminder: &super::ScheduledReminder,
    ) -> anyhow::Result<()> {
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
    match (current_state, event) {
        (ReminderState::Pending, ReminderEvent::Schedule) => {
            let delay = get_target_delay(&reminder.fire_at.time(), Utc::now())
                .to_std()
                .unwrap();

            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Scheduled)
                .await;

            task::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = tx.send(ReminderEvent::Trigger).await;
            });

            ReminderState::Scheduled
        }
        (ReminderState::Scheduled, ReminderEvent::Trigger) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Fired)
                .await;
            task::spawn(async move {
                tokio::time::sleep(NAGGING_TIMEOUT).await;
                let _ = tx.send(ReminderEvent::Trigger).await;
            });

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

            task::spawn(async move {
                tokio::time::sleep(NAGGING_TIMEOUT).await;
                let _ = tx.send(ReminderEvent::Trigger).await;
            });

            ReminderState::Nagging {
                attempts_left: attempts_left - 1,
            }
        }
        (ReminderState::Nagging { .. }, ReminderEvent::Acknowledge) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Acknowledge)
                .await;

            task::spawn(async move {
                tokio::time::sleep(CONFIRMATION_TIMEOUT).await;
                let _ = tx.send(ReminderEvent::Trigger).await;
            });

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

            task::spawn(async move {
                tokio::time::sleep(CONFIRMATION_TIMEOUT).await;
                let _ = tx.send(ReminderEvent::Trigger).await;
            });

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

#[derive(Debug)]
enum ReminderEvent {
    Schedule,
    Trigger,
    Acknowledge,
    Confirm,
    Stop,
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
