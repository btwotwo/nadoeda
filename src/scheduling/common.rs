use tokio::sync::mpsc;

use crate::reminder::Reminder;

#[derive(Debug)]
pub enum ReminderManagerMessage {
    Schedule(Reminder),
    ScheduleError(anyhow::Error, Reminder),
    ScheduleFinished(Reminder),
    Cancel(Reminder),
}

#[derive(Clone)]
pub struct ReminderManagerSender(mpsc::Sender<ReminderManagerMessage>);

impl ReminderManagerSender {
    pub fn new(inner: mpsc::Sender<ReminderManagerMessage>) -> Self {
        ReminderManagerSender(inner)
    }
    pub async fn schedule(&self, reminder: Reminder) -> anyhow::Result<()> {
        self.0
            .send(ReminderManagerMessage::Schedule(reminder))
            .await?;
        Ok(())
    }

    pub async fn notify_error(
        &self,
        error: anyhow::Error,
        reminder: Reminder,
    ) -> anyhow::Result<()> {
        self.0
            .send(ReminderManagerMessage::ScheduleError(error, reminder))
            .await?;

        Ok(())
    }
    pub async fn notify_completed(&self, reminder: Reminder) -> anyhow::Result<()> {
        self.0
            .send(ReminderManagerMessage::ScheduleFinished(reminder))
            .await?;

        Ok(())
    }
}

pub struct SchedulerContext {
    pub sender: ReminderManagerSender,
    pub reminder: Reminder,
}
