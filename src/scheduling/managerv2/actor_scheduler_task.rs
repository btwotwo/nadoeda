use std::collections::HashMap;

use tokio::{sync::mpsc, task::JoinHandle};

use crate::{reminder::ReminderId, scheduling::common::ReminderManagerMessage};

pub struct ActorReminderSchedulerTask {
    receiver: mpsc::Receiver<ReminderManagerMessage>,
    tasks: HashMap<ReminderId, JoinHandle<()>>,
}

impl ActorReminderSchedulerTask {
    async fn listen_to_messages(mut self) -> anyhow::Result<()> {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ReminderManagerMessage::Schedule(reminder) => todo!(),
                ReminderManagerMessage::WorkerError(error, reminder) => todo!(),
                ReminderManagerMessage::WorkerFinished(reminder) => todo!(),
                ReminderManagerMessage::Cancel(reminder) => todo!(),
            }
        }

        Ok(())
    }
}
