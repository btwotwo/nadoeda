#[derive(Debug)]
pub enum ReminderState {
    Pending,
    Scheduled,
    Nagging,
    Completed,
}

pub type ReminderId = u64;

#[derive(Debug)]
pub struct Reminder {
    pub id: ReminderId,
    pub state: ReminderState,
    pub fire_at: chrono::NaiveTime,
}
