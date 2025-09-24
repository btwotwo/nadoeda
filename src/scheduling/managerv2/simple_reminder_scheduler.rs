use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinHandle;

use crate::reminder::{Reminder, ReminderId};

use super::{scheduled_reminder_actor::get_target_delay, ReminderDeliveryChannel};

pub struct SimpleReminderScheduler {
    tasks: HashMap<ReminderId, JoinHandle<()>>
}

pub async fn run_reminder(mut reminder: Reminder, delivery: Arc<dyn ReminderDeliveryChannel>) {
    let delay = get_target_delay(&reminder.fire_at.time(), chrono::Utc::now().naive_utc()).to_std().unwrap();
    tokio::time::sleep(delay).await;
    
    delivery.send_reminder_notification(&reminder).await;
    
    todo!()
}
