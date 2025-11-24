
use std::error::Error;

use async_trait::async_trait;

use nadoeda_models::{
    reminder::{Reminder, ReminderFireTime, ReminderId},
    user::UserId,
};

pub struct NewReminder {
    pub text: String,
    pub fire_at: ReminderFireTime,
    pub user_id: UserId,
}

#[async_trait]
pub trait ReminderStorage: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn get(&self, id: ReminderId) -> Result<Option<Reminder>, Self::Error>;
    async fn get_all_user_reminders(&self, user_id: UserId) -> Result<Vec<Reminder>, Self::Error>;
    async fn insert(&self, reminder: NewReminder) -> Result<Reminder, Self::Error>;
    async fn update(&self, reminder: Reminder) -> Result<Reminder, Self::Error>;
}

// struct InMemoryReminderStore {
//     current_id: ReminderId,
//     storage: HashMap<ReminderId, Reminder>,
// }

// pub struct InMemoryReminderStorage {
//     store: RwLock<InMemoryReminderStore>,
// }

// impl InMemoryReminderStorage {

//     pub fn new() -> Self {
//         InMemoryReminderStorage {
//             store: RwLock::new(InMemoryReminderStore {
//                 current_id: 0,
//                 storage: HashMap::new(),
//             }),
//         }
//     }
// }

// #[async_trait]
// impl ReminderStorage for InMemoryReminderStorage {
//     type Error = anyhow::Error;

//     async fn insert(&self, reminder: NewReminder) -> Result<ReminderId, Self::Error> {
//         let mut store = self.store.write().await;
//         let current_id = store.current_id;
//         let storage = &mut store.storage;
//         let reminder_insert = Reminder {
//             fire_at: reminder.fire_at,
//             text: reminder.text,
//             state: ReminderState::Pending,
//             id: current_id,
//         };

//         storage.insert(current_id, reminder_insert);

//         store.current_id += 1;
//         log::info!("Returning current id {}", current_id);
//         Ok(current_id)
//     }

//     async fn update(&self, update_reminder: UpdateReminder) -> Result<ReminderId, Self::Error> {
//         let mut store = self.store.write().await;
//         let storage = &mut store.storage;
//         let id = update_reminder.id;
//         if let Some(mut reminder) = storage.remove(&id) {
//             reminder.text = update_reminder.text.unwrap_or(reminder.text);
//             reminder.fire_at = update_reminder.fire_at.unwrap_or(reminder.fire_at);
//             storage.insert(id, reminder);
//             Ok(id)
//         } else {
//             anyhow::bail!("Does not exist");
//         }
//     }

//     async fn get(&self, id: ReminderId) -> Result<Option<Reminder>, Self::Error> {
//         let store = self.store.read().await;
//         store.storage.get(&id).cloned()
//     }

//     async fn get_all(&self) -> Result<Option<Reminder>, Self::Error> {
//         let store = self.store.read().await;
//         store.storage.values().cloned().collect()
//     }
// }
