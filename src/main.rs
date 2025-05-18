#![allow(dead_code, unused_imports, unused_variables)]

use std::{collections::{HashMap, HashSet, VecDeque}, fmt::Debug};
use anyhow::ensure;
use chrono::{DateTime, Duration, Utc};
use diesel::connection::get_default_instrumentation;
use tokio::{task::JoinHandle, time::Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
enum ReminderState {
    Scheduled,
    Nagging,
    Completed
}

#[derive(Debug)]
struct Reminder {
    id: u64,
    state: ReminderState,
    fire_at: chrono::NaiveTime
}

struct ScheduleTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken
}

struct Scheduler {
    tasks: HashMap<u64, ScheduleTask>,
    cancellation_token: CancellationToken
}


impl Scheduler {
    pub fn schedule_reminder(&mut self, reminder: Reminder) -> anyhow::Result<()> {
        let cancellation_token = self.cancellation_token.clone();
        let now = Utc::now();
        let delay = Self::get_target_delay(&reminder, now).to_std().expect("The target delay is always in the future.");
        let task = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            println!("Reminder is firing! {:?}", reminder);
        });

        Ok(())
    }

    
    
    fn get_target_delay(reminder: &Reminder, now: DateTime<Utc>) -> Duration {
        todo!()
    }

    async fn do_work(previous_task: Option<ScheduleTask>, cancellation_token: CancellationToken, reminder: Reminder, delay: std::time::Duration) {
        if let Some(previous_task) = previous_task {
            Self::cancel_existing(previous_task).await;
        }
        
        tokio::select! {
            _ = cancellation_token.cancelled() => {println!("Task for scheduling reminder was cancelled. {:?}", reminder)},
            _ = tokio::time::sleep(delay) => {
                println!("Reminder is firing! {:?}", reminder);
            }
        }
    }

    async fn cancel_existing(schedule_task: ScheduleTask) {
        schedule_task.cancellation_token.cancel();
        schedule_task.task_handle.await.unwrap_or_else(|e| println!("Error while cancelling task. {:?}", e))
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
