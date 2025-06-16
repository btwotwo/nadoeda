use crate::reminder::{Reminder, ReminderId};

pub struct ReminderStorage {}

impl ReminderStorage {
    pub fn insert(&mut self, reminder: Reminder) -> anyhow::Result<ReminderId> {
        todo!()
    }

    pub fn update(&mut self, reminder: Reminder) -> anyhow::Result<ReminderId> {
        todo!()
    }
    
    pub fn get(&self, id: ReminderId) -> Option<Reminder> {
        todo!()
    }
}
