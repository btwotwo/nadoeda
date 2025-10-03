use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle},
};

use nadoeda_models::reminder::{Reminder, ReminderId, ReminderState};

use super::{ReminderDeliveryChannel, ReminderMessageType, ReminderSchedulerV2};

const NAGGING_ATTEMPTS: u8 = 10;
const NAGGING_TIMEOUT: Duration = Duration::from_secs(30);

const CONFIRMATION_ATTEMPTS: u8 = 10;
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

struct ScheduledReminderHandle {
    task: JoinHandle<()>,
    tx: mpsc::Sender<ReminderEvent>,
}

pub struct SimpleReminderScheduler {
    tasks: HashMap<ReminderId, ScheduledReminderHandle>,
}

impl SimpleReminderScheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }
}

impl ReminderSchedulerV2 for SimpleReminderScheduler {
    fn schedule_reminder(
        &mut self,
        schedule_request: super::ScheduleRequest,
        delivery_channel: Box<dyn ReminderDeliveryChannel>,
    ) -> anyhow::Result<super::ScheduledReminder> {
        let reminder_id = schedule_request.reminder.id;
        if let std::collections::hash_map::Entry::Vacant(e) = self.tasks.entry(reminder_id) {
            let (tx, rx) = mpsc::channel(10);
            let tx_clone = tx.clone();
            let task = task::spawn(async move {
                tx_clone.send(ReminderEvent::Schedule).await.unwrap();
                run_reminder(schedule_request.reminder, &delivery_channel, rx, tx_clone).await;
            });
            let scheduled_reminder = ScheduledReminderHandle { task, tx };
            e.insert(scheduled_reminder);
            Ok(super::ScheduledReminder { id: reminder_id })
        } else {
            anyhow::bail!("Already scheduled")
        }
    }

    fn cancel_reminder(
        &mut self,
        scheduled_reminder: super::ScheduledReminder,
    ) -> anyhow::Result<()> {
        if let Some((_, scheduled_reminder)) = self.tasks.remove_entry(&scheduled_reminder.id) {
            task::spawn(async move {
                scheduled_reminder
                    .tx
                    .send(ReminderEvent::Stop)
                    .await
                    .unwrap();
            });
            Ok(())
        } else {
            anyhow::bail!("No such reminder")
        }
    }
}

async fn run_reminder(
    mut reminder: Reminder,
    delivery: &Box<dyn ReminderDeliveryChannel>,
    mut rx: mpsc::Receiver<ReminderEvent>,
    tx: mpsc::Sender<ReminderEvent>,
) {
    // Todo: dispose of this task
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
    current: &ReminderState,
    event: &ReminderEvent,
    delivery: &Box<dyn ReminderDeliveryChannel>,
    tx: mpsc::Sender<ReminderEvent>,
) -> ReminderState {
    match (current, event) {
        (ReminderState::Pending, ReminderEvent::Schedule) => {
            let delay = get_target_delay(&reminder.fire_at.time(), Utc::now())
                .to_std()
                .unwrap();

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
            if *attempts_left <= 0 {
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
                let _ = tx.send(ReminderEvent::Trigger);
            });

            ReminderState::Nagging {
                attempts_left: attempts_left - 1,
            }
        }
        (ReminderState::Nagging { .. }, ReminderEvent::Confirm) => {
            delivery
                .send_reminder_notification(reminder, ReminderMessageType::Confirmation)
                .await;
            task::spawn(async move {
                tokio::time::sleep(CONFIRMATION_TIMEOUT).await;
                let _ = tx.send(ReminderEvent::Trigger);
            });

            ReminderState::Confirming {
                attempts_left: CONFIRMATION_ATTEMPTS,
            }
        }
        (ReminderState::Confirming { attempts_left }, ReminderEvent::Trigger) => {
            if *attempts_left <= 0 {
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
    Timeout,
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
mod target_datetime_tests {
    use super::*;

    use nadoeda_models::reminder::ReminderFireTime;
    use chrono::NaiveDate;
    use chrono::NaiveDateTime;
    use chrono::NaiveTime;
    use chrono::Timelike;
    use proptest::prelude::*;
    use proptest_arbitrary_interop::arb;

    #[test]
    pub fn when_firing_time_is_yet_to_come_target_delay_should_be_less_than_day() {
        let now_utc = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let now = DateTime::from_naive_utc_and_offset(now_utc, Utc);
        let fire_at = NaiveTime::from_hms_opt(13, 0, 0).unwrap();

        let delay = get_target_delay(&fire_at, now);

        assert_eq!(
            delay.num_hours(),
            1,
            "With given constraints the delay should be 1 hour."
        );
    }

    #[test]
    pub fn when_firing_time_is_passed_target_delay_should_be_next_day() {
        let now_utc = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let now = DateTime::from_naive_utc_and_offset(now_utc, Utc);

        let fire_at = ReminderFireTime::new(NaiveTime::from_hms_opt(11, 0, 0).unwrap());
        let delay = get_target_delay(&fire_at.time(), now);

        assert_eq!(
            delay.num_hours(),
            23,
            "With given constraints, the delay should be 23 hours"
        );
    }

    proptest! {
        #[test]
        fn test_target_delay(
            now_utc in arb::<NaiveDateTime>(),
            fire_at in arb::<NaiveTime>()
        ) {
            let fire_at = fire_at.with_nanosecond(0).unwrap();
            let now = DateTime::from_naive_utc_and_offset(now_utc.with_nanosecond(0).unwrap(), Utc);
            let delay = get_target_delay(&fire_at, now);
            let target_datetime = now + delay;

            assert!(target_datetime > now, "Target time should always be in the future");
            assert!(target_datetime.time() == fire_at, "Target time should be equal to fire_at time specified in the reminder. fire_at = {:?}, target_datetime.time() = {:?}, target_datetime = {:?}", fire_at, target_datetime.time(), target_datetime);
            assert!(delay.num_days() <= 1, "Delay should be one day or less. delay.days = {}", delay.num_days())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::{reminder::ReminderFireTime, scheduling::managerv2::ScheduleRequest};

    use super::*;
    type ReceivedMessages = Arc<Mutex<Vec<ReminderMessageType>>>;

    struct TestDeliveryChannel {
        received_messages: ReceivedMessages,
    }

    #[async_trait]
    impl ReminderDeliveryChannel for TestDeliveryChannel {
        async fn send_reminder_notification(
            &self,
            _reminder: &Reminder,
            message: ReminderMessageType,
        ) {
            self.received_messages.lock().unwrap().push(message);
        }
    }

    #[tokio::test(start_paused = true)]
    pub async fn scheduling_test() {
        let received_messages = received_messages();
        let delivery_channel = delivery_channel(&received_messages);
        let mut scheduler = SimpleReminderScheduler::new();
        let req = ScheduleRequest {
            reminder: reminder(NaiveTime::from_hms_milli_opt(12, 0, 0, 0).unwrap()),
        };
        let expected_delay = get_target_delay(&req.reminder.fire_at.time(), Utc::now());
        scheduler.schedule_reminder(req, delivery_channel).unwrap();

        wait_for_trigger(expected_delay).await;

        let msgs = received_messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(*msgs.first().unwrap(), ReminderMessageType::Fired);
    }

    #[tokio::test(start_paused = true)]
    pub async fn stopping_test() {
        let received_messages = received_messages();
        let delivery_channel = delivery_channel(&received_messages);
        let mut scheduler = SimpleReminderScheduler::new();
        let req = ScheduleRequest {
            reminder: reminder(NaiveTime::from_hms_milli_opt(12, 00, 00, 00).unwrap()),
        };
        let expected_delay = expected_delay(&req.reminder);

        let scheduled_reminder = scheduler.schedule_reminder(req, delivery_channel).unwrap();
        scheduler.cancel_reminder(scheduled_reminder).unwrap();

        wait_for_trigger(expected_delay).await;
        let msgs = received_messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(*msgs.first().unwrap(), ReminderMessageType::Stopped);

        wait_for_trigger(expected_delay).await;
    }

    async fn wait_for_trigger(expected_delay: chrono::Duration) {
        tokio::time::sleep(expected_delay.to_std().unwrap() + std::time::Duration::from_secs(15))
            .await
    }

    fn expected_delay(reminder: &Reminder) -> chrono::Duration {
        get_target_delay(&reminder.fire_at.time(), Utc::now())
    }

    fn reminder(fire_at: NaiveTime) -> Reminder {
        Reminder {
            id: 1,
            state: ReminderState::Pending,
            fire_at: ReminderFireTime::new(fire_at),
            text: "".to_string(),
        }
    }

    fn received_messages() -> ReceivedMessages {
        Arc::new(Mutex::new(vec![]))
    }

    fn delivery_channel(msgs: &ReceivedMessages) -> Box<TestDeliveryChannel> {
        Box::new(TestDeliveryChannel {
            received_messages: Arc::clone(msgs),
        })
    }
}
