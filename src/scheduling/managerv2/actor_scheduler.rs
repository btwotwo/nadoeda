use std::default;
use std::{collections::HashMap, sync::Arc};

use super::actor_scheduler_state::ActorReminderSchedulerState;
use super::scheduled_reminder_actor::{ScheduledReminderActor, ScheduledReminderActorReplyMessage};
use async_trait::async_trait;
use chrono::{NaiveDateTime, NaiveTime, TimeDelta};
use teloxide::types::ReplyMarkup;
use tokio::sync::{mpsc, oneshot};
use tokio::task::{self, JoinHandle};

use crate::actor::{self, ActorReference};
use crate::scheduling::managerv2::scheduled_reminder_actor::ScheduledReminderMessage;
use crate::{
    actor::{Actor, ActorContext, ActorHandle, ActorStatus},
    reminder::{self, Reminder, ReminderId},
    scheduling::common::ReminderManagerMessage,
};

use super::{ReminderSchedulerV2, ReminderWorkerV2, ScheduleRequest, ScheduledReminder};

pub enum ReminderManagerMessageV2 {
    ScheduleReminder {
        reminder: Reminder,
        worker: Box<dyn ReminderWorkerV2>,
    },
    ReminderScheduled {
        reminder_id: ReminderId,
        reminder_task: ActorHandle<ScheduledReminderActor>,
    },
    CancelReminder {
        reminder: ScheduledReminder,
    },
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
    ) -> anyhow::Result<ActorStatus<Self::State>> {
        let new_state = match msg {
            ReminderManagerMessageV2::ScheduleReminder { reminder, worker } => {
                task::spawn(async move {
                    let reminder_id = reminder.id;
                    let scheduled_reminder =
                        actor::start::<ScheduledReminderActor>(()).await.unwrap();
                    let (reply_sender, reply_receiver) = oneshot::channel();
                    let message = ScheduledReminderMessage::ScheduleStart {
                        reply_channel: reply_sender,
                        worker,
                        reminder,
                    };

                    scheduled_reminder.actor_reference().send_message(message);
                    context
                        .self_ref
                        .send_message(ReminderManagerMessageV2::ReminderScheduled {
                            reminder_task: scheduled_reminder,
                            reminder_id,
                        });
                    
                });
                
                state
            }
            ReminderManagerMessageV2::ReminderScheduled {
                reminder_id,
                reminder_task,
            } => {
                state.scheduled_reminders.insert(reminder_id, reminder_id)
            }
            ReminderManagerMessageV2::CancelReminder { reminder } => todo!(),
        };

        Ok(ActorStatus::Continue(new_state))
    }

    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State> {
        Ok(ActorReminderSchedulerState {
            scheduled_reminders: HashMap::new()
        })
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

        self.actor_handle.actor_reference().send_message(message);

        Ok(ScheduledReminder { id: reminder_id })
    }

    fn cancel_reminder(&mut self, scheduled_reminder: ScheduledReminder) -> anyhow::Result<()> {
        let message = ReminderManagerMessageV2::CancelReminder {
            reminder: scheduled_reminder,
        };

        self.actor_handle.actor_reference().send_message(message);

        Ok(())
    }
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
        scheduling::managerv2::{ScheduleRequest, scheduled_reminder_actor::get_target_delay},
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
