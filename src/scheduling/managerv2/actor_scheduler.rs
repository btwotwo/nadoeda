use std::collections::HashMap;

use super::
    actor_scheduler_state::{ActorReminderSchedulerState}
;
use async_trait::async_trait;
use chrono::{NaiveDateTime, NaiveTime, TimeDelta};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{
    actor::{Actor, ActorContext, ActorHandle},
    reminder::{self, Reminder, ReminderId},
    scheduling::common::ReminderManagerMessage,
};

use super::{ReminderSchedulerV2, ReminderWorkerV2, ScheduleRequest, ScheduledReminder};

pub enum ReminderManagerMessageV2 {
    ScheduleReminder {
        reminder: Reminder,
        worker: Box<dyn ReminderWorkerV2>,
    },
    CancelReminder {
        reminder: ScheduledReminder
    }
}

pub struct ActorReminderScheduler {
    actor_handle: ActorHandle<Self>,
}

#[async_trait]
impl Actor for ActorReminderScheduler {
    type Message = ReminderManagerMessageV2;
    type State = ActorReminderSchedulerState;
    type InitArgs = ();

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: &ActorContext<Self>,
    ) -> anyhow::Result<Self::State> {
        todo!()
    }

    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State> {
        todo!()
    }
}

impl ReminderSchedulerV2 for ActorReminderScheduler {
    fn schedule_reminder(
        &mut self,
        schedule_request: ScheduleRequest,
        worker: Box<dyn ReminderWorkerV2>,
    ) -> anyhow::Result<ScheduledReminder> {
        let reminder_id = schedule_request.reminder.id;
        let message = ReminderManagerMessageV2::ScheduleReminder {
            reminder: schedule_request.reminder,
            worker,
        };

        self.actor_handle.send_message(message);
        
        Ok(ScheduledReminder { id: reminder_id })
    }

    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()> {
        let message = ReminderManagerMessageV2::CancelReminder { reminder: scheduled_reminder };
        self.actor_handle.send_message(message);

        Ok(())
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
            .schedule_reminder(schedule_request, Box::new(worker))
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
            .schedule_reminder(schedule_request, Box::new(worker))
            .unwrap();
        tokio::time::sleep(target_delay.to_std().unwrap() - std::time::Duration::from_secs(60))
            .await;

        assert_eq!(*worker_hits.lock().unwrap(), 0)
    }

    fn get_worker() -> TestWorker {
        Default::default()
    }

    fn get_scheduler() -> ActorReminderScheduler {
        // ActorReminderScheduler
        todo!()
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
