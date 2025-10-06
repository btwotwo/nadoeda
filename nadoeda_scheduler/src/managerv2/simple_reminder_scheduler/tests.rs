mod target_datetime_tests;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nadoeda_models::reminder::ReminderFireTime;

use crate::managerv2::ScheduleRequest;

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

fn delivery_channel(msgs: &ReceivedMessages) -> TestDeliveryChannel {
    TestDeliveryChannel {
        received_messages: Arc::clone(msgs),
    }
}
