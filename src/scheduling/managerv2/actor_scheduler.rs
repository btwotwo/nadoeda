use chrono::{NaiveDateTime, NaiveTime, TimeDelta};

use super::{ReminderSchedulerV2, ReminderWorkerV2, ScheduleRequest, ScheduledReminder};

pub struct ActorReminderScheduler;

struct ActorReminderSchedulerState;

impl ActorReminderScheduler {
    pub fn start() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(128);
        todo!()
    }
}

impl ReminderSchedulerV2 for ActorReminderScheduler {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
        worker: impl ReminderWorkerV2,
    ) -> anyhow::Result<ScheduledReminder> {
        todo!()
    }

    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()> {
        todo!()
    }
}


fn get_target_delay(fire_at: &NaiveTime, now: NaiveDateTime) -> chrono::Duration {
    let max_delta = TimeDelta::new(10, 0).expect("This is always in bounds.");
    let delta = *fire_at - now.time();

    let today = now.date();
    let target_date = if delta <= max_delta {
        today
            .checked_add_signed(TimeDelta::days(1))
            .expect("Not realistic to overflow")
    } else {
        today
    };

    let target_datetime = target_date.and_time(*fire_at);

    target_datetime - now
}

#[cfg(test)]
mod tests {
    use std::{
        str::FromStr,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use chrono::Utc;

    use crate::{
        reminder::{Reminder, ReminderFireTime, ReminderState},
        scheduling::managerv2::ScheduleRequest,
    };

    use super::*;

    #[derive(Default)]
    struct TestWorker {
        hits: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl ReminderWorkerV2 for TestWorker {
        async fn handle_reminder(&self, _reminder: &Reminder) -> anyhow::Result<()> {
            *self.hits.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_calls_worker_after_delay() {
        let worker = get_worker();
        let worker_hits = Arc::clone(&worker.hits);
        let mut scheduler = get_scheduler();
        let reminder = get_reminder();
        let target_delay = get_target_delay(&reminder.fire_at.time(), Utc::now().naive_utc());
        let schedule_request = ScheduleRequest { reminder };

        scheduler
            .schedule_reminder(schedule_request, worker)
            .unwrap();
        tokio::time::sleep(target_delay.to_std().unwrap()).await;

        assert_eq!(*worker_hits.lock().unwrap(), 1)
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_does_not_call_worker_before_delay() {
        let worker = get_worker();
        let worker_hits = Arc::clone(&worker.hits);
        let mut scheduler = get_scheduler();
        let reminder = get_reminder();
        let target_delay = get_target_delay(&reminder.fire_at.time(), Utc::now().naive_utc());
        let schedule_request = ScheduleRequest { reminder };

        scheduler
            .schedule_reminder(schedule_request, worker)
            .unwrap();
        tokio::time::sleep(target_delay.to_std().unwrap() - std::time::Duration::from_secs(60))
            .await;

        assert_eq!(*worker_hits.lock().unwrap(), 0)
    }

    fn get_worker() -> TestWorker {
        Default::default()
    }

    fn get_scheduler() -> ActorReminderScheduler {
        ActorReminderScheduler
    }

    fn get_reminder() -> Reminder {
        Reminder {
            id: 0,
            state: ReminderState::Pending,
            fire_at: ReminderFireTime::new(chrono::NaiveTime::from_str("12:00").unwrap()),
            text: "Test".to_string(),
        }
    }
}
