use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

use nadoeda_models::reminder::{Reminder, ReminderId, ReminderState};

use super::{NewReminder, model::UpdateReminder};

#[async_trait]
pub trait ReminderStorage: Send + Sync {
    async fn insert(&self, reminder: NewReminder) -> anyhow::Result<ReminderId>;
    async fn update(&self, reminder: UpdateReminder) -> anyhow::Result<ReminderId>;
    async fn get(&self, id: ReminderId) -> Option<Reminder>;
    async fn get_all(&self) -> Vec<Reminder>;
}

struct InMemoryReminderStore {
    current_id: ReminderId,
    storage: HashMap<ReminderId, Reminder>,
}

pub struct InMemoryReminderStorage {
    store: RwLock<InMemoryReminderStore>,
}

impl InMemoryReminderStorage {
    pub fn new() -> Self {
        InMemoryReminderStorage {
            store: RwLock::new(InMemoryReminderStore {
                current_id: 0,
                storage: HashMap::new(),
            }),
        }
    }
}

#[async_trait]
impl ReminderStorage for InMemoryReminderStorage {
    async fn insert(&self, reminder: NewReminder) -> anyhow::Result<ReminderId> {
        let mut store = self.store.write().await;
        let current_id = store.current_id;
        let storage = &mut store.storage;
        let reminder_insert = Reminder {
            fire_at: reminder.fire_at,
            text: reminder.text,
            state: ReminderState::Pending,
            id: current_id,
        };

        storage.insert(current_id, reminder_insert);

        store.current_id += 1;
        log::info!("Returning current id {}", current_id);
        Ok(current_id)
    }

    async fn update(&self, update_reminder: UpdateReminder) -> anyhow::Result<ReminderId> {
        let mut store = self.store.write().await;
        let storage = &mut store.storage;
        let id = update_reminder.id;
        if let Some(mut reminder) = storage.remove(&id) {
            reminder.text = update_reminder.text.unwrap_or(reminder.text);
            reminder.fire_at = update_reminder.fire_at.unwrap_or(reminder.fire_at);
            storage.insert(id, reminder);
            Ok(id)
        } else {
            anyhow::bail!("Does not exist");
        }
    }

    async fn get(&self, id: ReminderId) -> Option<Reminder> {
        let store = self.store.read().await;
        store.storage.get(&id).cloned()
    }

    async fn get_all(&self) -> Vec<Reminder> {
        let store = self.store.read().await;
        store.storage.values().cloned().collect()
    }
}
