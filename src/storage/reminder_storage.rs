use std::{collections::{HashMap, HashSet}, sync::Arc};

use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::reminder::{Reminder, ReminderId};

#[async_trait]
pub trait ReminderStorage {
    async fn insert(&self, reminder: Reminder) -> anyhow::Result<ReminderId>;
    async fn update(&self, reminder: Reminder) -> anyhow::Result<ReminderId>;
    async fn get(&self, id: ReminderId) -> Option<Reminder>;
}

#[derive(Clone)]
pub struct InMemoryReminderStorage {
    store: Arc<RwLock<(ReminderId, HashMap<ReminderId, Reminder>)>>,
}

impl InMemoryReminderStorage {
    pub fn new() -> Self {
        InMemoryReminderStorage {
            store: Arc::new(RwLock::new((0, HashMap::new())))
        }
    }
}

#[async_trait]
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
