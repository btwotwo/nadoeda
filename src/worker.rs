use crate::common::SchedulerContext;

pub trait ReminderWorker {
    fn handle_reminder(
        &self,
        context: &SchedulerContext,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait WorkerFactory {
    type Worker: ReminderWorker;

    fn create_worker(&self) -> Self::Worker;
}
