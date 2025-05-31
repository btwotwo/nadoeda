use tokio::sync::mpsc;

use crate::reminder::Reminder;

#[derive(Debug)]
pub enum ReminderManagerMessage {
    Schedule(Reminder),
    Cancel(Reminder),
}

pub type ReminderManagerSender = mpsc::Sender<ReminderManagerMessage>;

pub struct SchedulerContext {
    pub sender: ReminderManagerSender,
    pub reminder: Reminder,
}
