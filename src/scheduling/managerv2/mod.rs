mod simple_reminder_scheduler;

use async_trait::async_trait;

use crate::reminder::{Reminder, ReminderId};

pub struct ScheduleRequest {
    reminder: Reminder,
}

pub struct ScheduledReminder {
    id: ReminderId,
}

#[async_trait]
pub trait ReminderWorkerV2: Send + 'static {
    async fn handle_reminder(&self, reminder: &Reminder) -> anyhow::Result<()>;
}

#[async_trait]
pub trait ReminderDeliveryChannel : Send + Sync + 'static {
    async fn send_reminder_notification(&self, reminder: &Reminder);
}

pub trait ReminderSchedulerHandle {
    fn notify_success();
    fn notify_error(e: anyhow::Error);
}

pub trait ReminderSchedulerV2: Send + Sync + 'static {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
        worker: Box<dyn ReminderWorkerV2>,
    ) -> anyhow::Result<ScheduledReminder>;

    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()>;
}

