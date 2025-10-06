mod target_datetime_tests;

use std::sync::{Arc, Mutex};

use crate::managerv2::ScheduleRequest;
use async_trait::async_trait;
use nadoeda_models::reminder::ReminderFireTime;
use proptest::prelude::*;
use test_strategy::proptest;

use super::*;
type ReceivedMessages = Arc<Mutex<Vec<ReminderMessageType>>>;

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
    pub delivery_channel: TestDeliveryChannel,
    pub scheduler: SimpleReminderScheduler,
}

impl TestContext {
    fn new() -> Self {
        let received_messages = received_messages();
        let delivery_channel = delivery_channel(&received_messages);
        let scheduler = SimpleReminderScheduler::new();

        Self {
            received_messages: received_messages.clone(),
            delivery_channel,
            scheduler,
        }
    }
}

fn tokio_ct(future: impl Future<Output = Result<(), TestCaseError>>) -> Result<(), TestCaseError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(future)
}

#[proptest(async=tokio_ct)]
async fn scheduling_proptest(#[strategy(0..24u32)] hours: u32, #[strategy(0..60u32)] minutes: u32) {
    let mut ctx = TestContext::new();
    let req = schedule_request(hours, minutes);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler
        .schedule_reminder(req, ctx.delivery_channel) 
        .unwrap();

    wait(expected_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    prop_assert_eq!(msgs.len(), 1);
    prop_assert_eq!(msgs[0], ReminderMessageType::Fired);
}

#[proptest(async=tokio_ct)]
async fn stopping_proptest(#[strategy(0..24u32)] hours: u32, #[strategy(0..60u32)] minutes: u32) {
        let mut ctx = TestContext::new();
    let req = schedule_request(hours, minutes);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx
        .scheduler
        .schedule_reminder(req, ctx.delivery_channel)
        .unwrap();
    ctx.scheduler.cancel_reminder(scheduled_reminder).await.unwrap();

    wait(expected_delay).await;
    let msgs = ctx.received_messages.lock().unwrap();

    prop_assert_eq!(msgs.len(), 1);
    prop_assert_eq!(msgs[0], ReminderMessageType::Stopped);

    wait(expected_delay).await;
}

#[proptest(async=tokio_ct)]
async fn nagging_proptest(#[strategy(0..24u32)] hours: u32, #[strategy(0..60u32)] minutes: u32) {
    let mut ctx = TestContext::new();
    let req = schedule_request(hours, minutes);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler
        .schedule_reminder(req, ctx.delivery_channel)
        .unwrap();

    wait(expected_delay).await;

    let expected_nagging_delay = chrono::Duration::from_std(NAGGING_TIMEOUT).unwrap();

    wait(expected_nagging_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    
    prop_assert_eq!(msgs.len(), 2);
    prop_assert_eq!(msgs[0], ReminderMessageType::Fired);
    prop_assert_eq!(msgs[1], ReminderMessageType::Nag);
}

#[proptest(async=tokio_ct)]
async fn confirmation_proptest(#[strategy(0..24u32)] hours: u32, #[strategy(0..60u32)] minutes: u32) {
    let mut ctx = TestContext::new();
    let req = schedule_request(hours, minutes);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req, ctx.delivery_channel).unwrap();

    wait(expected_delay).await;

    ctx.scheduler.acknowledge_reminder(scheduled_reminder).await.unwrap();
    let expected_confirmation_delay = chrono::Duration::from_std(CONFIRMATION_TIMEOUT - Duration::from_secs(1)).unwrap();

    wait(expected_confirmation_delay).await;
    
    let msgs = ctx.received_messages.lock().unwrap();

    assert_eq!(*msgs, vec![ReminderMessageType::Fired, ReminderMessageType::Acknowledge, ReminderMessageType::Confirmation]);
}


#[tokio::test(start_paused = true)]
pub async fn scheduling_test() {
    let mut ctx = TestContext::new();
    let req = schedule_request(12, 00);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler
        .schedule_reminder(req, ctx.delivery_channel)
        .unwrap();

    wait(expected_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], ReminderMessageType::Fired);
}

#[tokio::test(start_paused = true)]
pub async fn stopping_test() {
    let mut ctx = TestContext::new();
    let req = schedule_request(12, 00);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx
        .scheduler
        .schedule_reminder(req, ctx.delivery_channel)
        .unwrap();
    
    ctx.scheduler.cancel_reminder(scheduled_reminder).await.unwrap();

    wait(expected_delay).await;
    let msgs = ctx.received_messages.lock().unwrap();

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], ReminderMessageType::Stopped);

    wait(expected_delay).await;
}

#[tokio::test(start_paused = true)]
pub async fn nagging_test() {
    let mut ctx = TestContext::new();
    let req = schedule_request(12, 00);
    let expected_delay = expected_delay(&req.reminder);

    ctx.scheduler
        .schedule_reminder(req, ctx.delivery_channel)
        .unwrap();

    wait(expected_delay).await;

    let expected_nagging_delay = chrono::Duration::from_std(NAGGING_TIMEOUT).unwrap();

    wait(expected_nagging_delay).await;

    let msgs = ctx.received_messages.lock().unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0], ReminderMessageType::Fired);
    assert_eq!(msgs[1], ReminderMessageType::Nag)
}

#[tokio::test(start_paused = true)]
pub async fn confirmation_test() {
    let mut ctx = TestContext::new();
    let req = schedule_request(12, 00);
    let expected_delay = expected_delay(&req.reminder);

    let scheduled_reminder = ctx.scheduler.schedule_reminder(req, ctx.delivery_channel).unwrap();

    wait(expected_delay).await;

    ctx.scheduler.acknowledge_reminder(scheduled_reminder).await.unwrap();
    let expected_confirmation_delay = chrono::Duration::from_std(CONFIRMATION_TIMEOUT).unwrap();

    wait(expected_confirmation_delay * 2).await;
    
    let msgs = ctx.received_messages.lock().unwrap();

    assert_eq!(*msgs, vec![ReminderMessageType::Fired, ReminderMessageType::Acknowledge, ReminderMessageType::Confirmation, ReminderMessageType::Confirmation]);
}

async fn wait(expected_delay: chrono::Duration) {
    tokio::time::sleep(expected_delay.to_std().unwrap() + std::time::Duration::from_secs(15)).await
}

fn expected_delay(reminder: &Reminder) -> chrono::Duration {
    get_target_delay(&reminder.fire_at.time(), Utc::now())
}

fn received_messages() -> ReceivedMessages {
    Arc::new(Mutex::new(vec![]))
}

fn delivery_channel(msgs: &ReceivedMessages) -> TestDeliveryChannel {
    TestDeliveryChannel {
        received_messages: Arc::clone(msgs),
    }
}

fn reminder_at(time: NaiveTime) -> Reminder {
    Reminder {
        id: 1,
        state: ReminderState::Pending,
        fire_at: ReminderFireTime::new(time),
        text: "Reminder Text".to_owned(),
    }
}

fn schedule_request(hours: u32, minutes: u32) -> ScheduleRequest {
    ScheduleRequest {
        reminder: reminder_at(NaiveTime::from_hms_milli_opt(hours, minutes, 0, 0).unwrap()),
    }
}
