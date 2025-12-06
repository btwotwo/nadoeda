use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Error;
use async_trait::async_trait;
use nadoeda_scheduler::{ReminderScheduler, ScheduleRequest, ScheduledReminder};
use nadoeda_storage::sqlite::{
    reminder_storage::SqliteReminderStorage, user_storage::SqliteUserInfoStorage,
};
use sqlx::{Pool, Sqlite};
use teloxide::{
    dispatching::{DpHandlerDescription, UpdateHandler},
    dptree::Handler,
    types::ChatId,
};
use teloxide_tests::{MockBot, MockMessageText, mock_bot::DistributionKey};

use crate::HandlerResult;

pub struct NoopReminderScheduler;
#[async_trait]
impl ReminderScheduler for NoopReminderScheduler {
    async fn schedule_reminder(
        &self,
        schedule_request: ScheduleRequest,
    ) -> anyhow::Result<ScheduledReminder> {
        Ok(ScheduledReminder::new(1))
    }

    async fn cancel_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        Ok(())
    }

    async fn acknowledge_reminder(
        &self,
        scheduled_reminder: &ScheduledReminder,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn confirm_reminder(&self, scheduled_reminder: &ScheduledReminder) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct CallMarker(Arc<AtomicBool>);

impl CallMarker {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn set_called(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn was_called(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub async fn call_marker_endpoint(called: CallMarker) -> HandlerResult {
    called.set_called();
    Ok(())
}

pub fn storage(pool: Pool<Sqlite>) -> Arc<SqliteReminderStorage> {
    Arc::new(SqliteReminderStorage::new(pool))
}

pub fn user_storage(pool: Pool<Sqlite>) -> Arc<SqliteUserInfoStorage> {
    Arc::new(SqliteUserInfoStorage::new(pool))
}

pub fn bot(
    msg_text: &str,
    schema: Handler<'static, Result<(), Error>, DpHandlerDescription>,
) -> (MockBot<anyhow::Error, DistributionKey>, ChatId) {
    let mock_text = MockMessageText::new().text(msg_text);
    let chat_id = mock_text.chat.id.clone();
    let bot = MockBot::new(mock_text, schema);

    (bot, chat_id)
}
