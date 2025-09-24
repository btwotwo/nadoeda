use async_trait::async_trait;
use chrono::{NaiveDateTime, NaiveTime, TimeDelta};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    actor::{Actor, ActorContext, ActorReference, ActorStatus},
    reminder::{self, Reminder},
    scheduling::ReminderWorker,
};

use super::ReminderWorkerV2;

pub struct ScheduledReminderActor;

pub type ScheduledReminderActorReplyMessage = anyhow::Result<()>;

pub enum ScheduledReminderMessage {
    ScheduleStart {
        reply_channel: oneshot::Sender<ScheduledReminderActorReplyMessage>,
        worker: Box<dyn ReminderWorkerV2>,
        reminder: Reminder,
    },
    ScheduleFinish,
}

#[async_trait]
impl Actor for ScheduledReminderActor {
    type Message = ScheduledReminderMessage;
    type State = Option<JoinHandle<()>>;
    type InitArgs = ();

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: ActorContext<Self>,
    ) -> anyhow::Result<ActorStatus<Self::State>> {
        let self_ref = context.self_ref.clone();
        match msg {
            ScheduledReminderMessage::ScheduleStart {
                reply_channel,
                worker,
                reminder,
            } => {
                let target_delay =
                    get_target_delay(&reminder.fire_at.time(), chrono::Utc::now().naive_utc())
                        .to_std()
                        .unwrap();

                let task_handle = tokio::spawn(async move {
                    tokio::time::sleep(target_delay).await;
                    worker.handle_reminder(&reminder).await.unwrap();
                    reply_channel.send(Ok(())).unwrap();
                    self_ref.send_message(ScheduledReminderMessage::ScheduleFinish)
                });

                Ok(ActorStatus::Continue(Some(task_handle)))
            }

            ScheduledReminderMessage::ScheduleFinish => Ok(ActorStatus::Stop),
        }
    }

    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State> {
        Ok(None)
    }
}


pub(crate) fn get_target_delay(fire_at: &NaiveTime, now: NaiveDateTime) -> chrono::Duration {
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
