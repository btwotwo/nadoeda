use std::marker::PhantomData;

use async_trait::async_trait;
use nadoeda_models::reminder::Reminder;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReminderMessageType {
    Scheduled,
    Fired,
    Nag,
    Confirmation,
    Acknowledge,
    Timeout,
    Finished,
    Stopped,
}

#[async_trait]
pub trait ReminderDeliveryChannel: Send + Sync + 'static {
    async fn send_reminder_notification(&self, reminder: &Reminder, message: ReminderMessageType);
}
