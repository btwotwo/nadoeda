use std::collections::HashMap;

use crate::{actor::ActorHandle, reminder::ReminderId};

use super::scheduled_reminder_actor::ScheduledReminderActor;

pub struct ActorReminderSchedulerState {
    pub scheduled_reminders: HashMap<ReminderId, ActorHandle<ScheduledReminderActor>>
}
