use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use tokio::sync::RwLock;

use crate::reminder::{Reminder, ReminderId};

trait ReminderStorage {
    async fn insert(&self, reminder: Reminder) -> anyhow::Result<ReminderId>;
    async fn update(&self, reminder: Reminder) -> anyhow::Result<ReminderId>;
    async fn get(&self, id: ReminderId) -> Option<Reminder>;
}

pub struct InMemoryReminderStorage {
    store: RwLock<(ReminderId, HashMap<ReminderId, Reminder>)>,
}

impl InMemoryReminderStorage {
    pub fn new() -> Self {
        InMemoryReminderStorage {
            store: RwLock::new((0, HashMap::new()))
        }
    }
}

impl ReminderStorage for InMemoryReminderStorage {
    async fn insert(&self, mut reminder: Reminder) -> anyhow::Result<ReminderId> {
        let mut store = self.store.write().await;
        let current_id = store.0;
        let storage = &mut store.1;

        if storage.contains_key(&reminder.id) {
            anyhow::bail!("Already exists");
        }

        reminder.id = current_id;
        storage.insert(current_id, reminder);

        store.0 += 1;

        Ok(current_id)
    }

    async fn update(&self, reminder: Reminder) -> anyhow::Result<ReminderId> {
        let mut store = self.store.write().await;
        let storage = &mut store.1;
        let id = reminder.id;
        if let Some(old_reminder) = storage.get(&id) {
            storage.insert(id, reminder);
            Ok(id)
        } else {
            anyhow::bail!("Does not exist");
        }
    }

    async fn get(&self, id: ReminderId) -> Option<Reminder> {
        let store = self.store.read().await;
        store.1.get(&1).cloned()
    }
}
