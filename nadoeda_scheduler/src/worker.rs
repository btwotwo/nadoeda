use async_trait::async_trait;

use super::common::SchedulerContext;

#[async_trait]
pub trait ReminderWorker: Send + 'static {
    async fn handle_reminder(&self, context: &SchedulerContext) -> anyhow::Result<()>;
}

pub trait WorkerFactory: Send {
    type Worker: ReminderWorker;

    fn create_worker(&self) -> Self::Worker;
}
