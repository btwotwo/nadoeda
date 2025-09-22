use async_trait::async_trait;
use chrono::{NaiveDateTime, NaiveTime, TimeDelta};
use tokio::sync::{mpsc, oneshot};

use crate::{
    actor::{Actor, ActorContext, ActorStatus},
    reminder::Reminder,
};

use super::ReminderWorkerV2;

struct ScheduledReminderActor;

type ReplyMessage = anyhow::Result<()>;

enum ScheduledReminderMessage {
    ScheduleStart {
        reply_channel: oneshot::Sender<ReplyMessage>,
        worker: Box<dyn ReminderWorkerV2>,
        reminder: Reminder,
    },
}

#[async_trait]
impl Actor for ScheduledReminderActor {
    type Message = ScheduledReminderMessage;
    type State = ();
    type InitArgs = ();

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: &ActorContext<Self>,
    ) -> anyhow::Result<ActorStatus<Self::State>> {
        match msg {
            ScheduledReminderMessage::ScheduleStart {
                reply_channel,
                worker,
                reminder,
            } => {
                let target_delay =
                    get_target_delay(&reminder.fire_at.time(), chrono::Utc::now().naive_utc()).to_std().unwrap();
                tokio::spawn(async move { tokio::time::sleep(target_delay).await });
            }
        }
        Ok(ActorStatus::Continue(()))
    }

    async fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State> {
        Ok(())
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
