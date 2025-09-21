use async_trait::async_trait;

use crate::actor::Actor;

struct ScheduledReminderActor;
enum ScheduledReminderMessage {
    ScheduleStart,
}

#[async_trait]
impl Actor for ScheduledReminderActor {
    type Message = ScheduledReminderActor;
    type State = ();
    type InitArgs = ();

    fn handle_message(
        msg: Self::Message,
        state: Self::State,
        context: &ActorContext<Self>,
    ) -> anyhow::Result<Self::State> {
        todo!()
    }

    fn init_state(args: Self::InitArgs) -> anyhow::Result<Self::State> {
        todo!()
    }
}
