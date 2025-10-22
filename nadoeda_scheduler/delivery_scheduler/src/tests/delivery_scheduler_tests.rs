use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::ReminderMessageType;
use async_trait::async_trait;
use chrono::{NaiveTime, Utc};
use nadoeda_models::reminder::{Reminder, ReminderFireTime, ReminderState};
use proptest::prelude::*;
use test_strategy::proptest;

use super::*;

type ReceivedMessages = Arc<Mutex<Vec<ReminderMessageType>>>;

#[derive(Clone)]
struct TestDeliveryChannel {
    received_messages: ReceivedMessages,
}

#[async_trait]
impl ReminderDeliveryChannel for TestDeliveryChannel {
    async fn send_reminder_notification(&self, _reminder: &Reminder, message: ReminderMessageType) {
        self.received_messages.lock().unwrap().push(message);
    }
}

struct TestContext {
    pub received_messages: ReceivedMessages,
    pub scheduler: DeliveryReminderScheduler,
}

impl TestContext {
    fn new() -> Self {
        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let delivery_channel = TestDeliveryChannel {
            received_messages: received_messages.clone(),
        };
        let scheduler = DeliveryReminderScheduler::new(Arc::new(delivery_channel.clone()));

        Self {
            received_messages,
            scheduler,
        }
    }
}

fn time_strategy() -> impl Strategy<Value = NaiveTime> {
    (0u32..24, 0u32..60).prop_map(|(h, m)| NaiveTime::from_hms_opt(h, m, 0).unwrap())
}

fn tokio_ct(
    future: impl std::future::Future<Output = Result<(), TestCaseError>>,
) -> Result<(), TestCaseError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(future)
}

#[proptest(async = tokio_ct)]
async fn scheduling_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert_eq!(
        &msgs[..],
        &[ReminderMessageType::Scheduled, ReminderMessageType::Fired]
    );
}

#[proptest(async = tokio_ct)]
async fn stopping_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req).await.unwrap();

    ctx.scheduler
        .cancel_reminder(&scheduled_reminder)
        .await
        .unwrap();

    wait(expected_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert_eq!(&msgs[..], &[ReminderMessageType::Stopped]);
}

#[proptest(async = tokio_ct)]
async fn nagging_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;
    wait(chrono::Duration::from_std(NAGGING_TIMEOUT).unwrap()).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert!(msgs.len() >= 3);
    prop_assert_eq!(msgs[0], ReminderMessageType::Scheduled);
    prop_assert_eq!(msgs[1], ReminderMessageType::Fired);
    prop_assert_eq!(*msgs.last().unwrap(), ReminderMessageType::Nag);
}

#[proptest(async = tokio_ct)]
async fn confirmation_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;

    ctx.scheduler
        .acknowledge_reminder(&scheduled_reminder)
        .await
        .unwrap();

    wait(chrono::Duration::from_std(CONFIRMATION_TIMEOUT - Duration::from_secs(1)).unwrap()).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert!(msgs.len() >= 4, "msgs.len() = {}", msgs.len());
    prop_assert_eq!(msgs[0], ReminderMessageType::Scheduled);
    prop_assert_eq!(msgs[1], ReminderMessageType::Fired);
    prop_assert_eq!(msgs[2], ReminderMessageType::Acknowledge);
    prop_assert_eq!(*msgs.last().unwrap(), ReminderMessageType::Confirmation);
}

#[proptest(async = tokio_ct)]
async fn finish_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;

    ctx.scheduler
        .acknowledge_reminder(&scheduled_reminder)
        .await
        .unwrap();

    wait(chrono::Duration::from_std(CONFIRMATION_TIMEOUT - Duration::from_secs(1)).unwrap()).await;

    ctx.scheduler
        .confirm_reminder(&scheduled_reminder)
        .await
        .unwrap();

    wait(expected_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert!(msgs.len() >= 5, "msgs.len() = {}", msgs.len());
    prop_assert_eq!(msgs[0], ReminderMessageType::Scheduled);
    prop_assert_eq!(msgs[1], ReminderMessageType::Fired);
    prop_assert_eq!(msgs[2], ReminderMessageType::Acknowledge);
    prop_assert_eq!(msgs[3], ReminderMessageType::Confirmation);
    prop_assert_eq!(*msgs.last().unwrap(), ReminderMessageType::Finished);
}

#[proptest(async = tokio_ct)]
async fn nagging_timeout_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;

    let total_nagging_time =
        chrono::Duration::from_std(NAGGING_TIMEOUT * NAGGING_ATTEMPTS as u32).unwrap();

    wait(total_nagging_time * 2).await; // Very long time

    let msgs = ctx.received_messages.lock().unwrap();

    let nag_count = msgs
        .iter()
        .filter(|i| matches!(i, ReminderMessageType::Nag))
        .count();

    prop_assert_eq!(nag_count, NAGGING_ATTEMPTS as usize);
    prop_assert_eq!(*msgs.last().unwrap(), ReminderMessageType::Timeout);
}

#[proptest(async = tokio_ct)]
async fn confirmation_timeout_proptest(#[strategy(time_strategy())] time: NaiveTime) {
    let ctx = TestContext::new();
    let req = schedule_request(time);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req).await.unwrap();

    wait(expected_delay).await;

    ctx.scheduler
        .acknowledge_reminder(&scheduled_reminder)
        .await
        .unwrap();

    let total_confirmation_time =
        chrono::Duration::from_std(CONFIRMATION_TIMEOUT * CONFIRMATION_ATTEMPTS as u32).unwrap();

    wait(total_confirmation_time).await;

    let msgs = ctx.received_messages.lock().unwrap();

    let confirmation_count = msgs
        .iter()
        .filter(|i| matches!(i, ReminderMessageType::Confirmation))
        .count();

    prop_assert_eq!(confirmation_count, CONFIRMATION_ATTEMPTS as usize);
    prop_assert_eq!(*msgs.last().unwrap(), ReminderMessageType::Timeout);
}

async fn wait(duration: chrono::Duration) {
    tokio::time::sleep(duration.to_std().unwrap() + std::time::Duration::from_secs(1)).await;
}

fn expected_delay(reminder: &Reminder) -> chrono::Duration {
    get_target_delay(&reminder.fire_at.time(), Utc::now())
}

fn reminder_at(time: NaiveTime) -> Reminder {
    Reminder {
        id: 1,
        state: ReminderState::Pending,
        fire_at: ReminderFireTime::new(time),
        text: "Reminder Text".to_owned(),
    }
}

fn schedule_request(time: NaiveTime) -> ScheduleRequest {
    ScheduleRequest {
        reminder: reminder_at(time),
    }
}
