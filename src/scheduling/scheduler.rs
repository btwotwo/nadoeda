use chrono::{Duration, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use tokio::{task::JoinHandle, time};
use tokio_util::sync::CancellationToken;

use super::{common::SchedulerContext, worker::ReminderWorker};

pub struct ScheduledTask {
    task_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl ScheduledTask {
    pub fn new(task_handle: JoinHandle<()>, cancellation_token: CancellationToken) -> Self {
        Self {
            task_handle,
            cancellation_token,
        }
    }
    pub async fn cancel(self, timeout: std::time::Duration) {
        self.cancellation_token.cancel();
        let cancel_with_timeout = time::timeout(timeout, self.task_handle);
        let _ = cancel_with_timeout.await;
    }
}

pub struct ReminderScheduler;

impl ReminderScheduler {
    pub fn schedule_reminder(
        context: SchedulerContext,
        worker: impl ReminderWorker + Send + 'static,
    ) -> ScheduledTask {
        let cancellation_token = CancellationToken::new();
        let task_cancellation_token = cancellation_token.child_token();

        let reminder_id = context.reminder.id;

        let now = Utc::now().naive_utc();
        let delay = Self::get_target_delay(&context.reminder.fire_at.time(), now)
            .to_std()
            .expect("The target delay is always in the future.");

        let task_handle = tokio::spawn(async move {
            let result =
                Self::handle_reminder_after_delay(task_cancellation_token, &context, delay, worker)
                    .await;
            match result {
                Ok(_) => context
                    .sender
                    .notify_completed(context.reminder)
                    .await
                    .expect("Could not notify parent"),
                Err(error) => context
                    .sender
                    .notify_error(error, context.reminder)
                    .await
                    .expect("Could not notify parent."),
            }
        });

        ScheduledTask::new(task_handle, cancellation_token)
    }

    async fn handle_reminder_after_delay<TWorker: ReminderWorker + Send>(
        cancellation_token: CancellationToken,
        ctx: &SchedulerContext,
        delay: std::time::Duration,
        worker: TWorker,
    ) -> anyhow::Result<()> {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                println!("Task for scheduling reminder was cancelled. {:?}", ctx.reminder);
            },
            _ = tokio::time::sleep(delay) => {
                worker.handle_reminder(ctx).await?;
            }
        };
        Ok(())
    }

    pub(super) fn get_target_delay(fire_at: &NaiveTime, now: NaiveDateTime) -> Duration {
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
}

#[cfg(test)]
mod tests {
    use crate::reminder::ReminderFireTime;
    use crate::scheduling::scheduler::ReminderScheduler;
    use chrono::NaiveDate;
    use chrono::NaiveDateTime;
    use chrono::NaiveTime;
    use chrono::Timelike;
    use proptest::prelude::*;
    use proptest_arbitrary_interop::arb;

    #[test]
    pub fn when_firing_time_is_yet_to_come_target_delay_should_be_less_than_day() {
        let now = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let fire_at = NaiveTime::from_hms_opt(13, 0, 0).unwrap();

        let delay = ReminderScheduler::get_target_delay(&fire_at, now);

        assert_eq!(
            delay.num_hours(),
            1,
            "With given constraints the delay should be 1 hour."
        );
    }

    #[test]
    pub fn when_firing_time_is_passed_target_delay_should_be_next_day() {
        let now = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let fire_at = ReminderFireTime::new(NaiveTime::from_hms_opt(11, 0, 0).unwrap());
        let delay = ReminderScheduler::get_target_delay(&fire_at.time(), now);

        assert_eq!(
            delay.num_hours(),
            23,
            "With given constraints, the delay should be 23 hours"
        );
    }

    proptest! {
        #[test]
        fn test_target_delay(
            now in arb::<NaiveDateTime>(),
            fire_at in arb::<NaiveTime>()
        ) {
            let fire_at = fire_at.with_nanosecond(0).unwrap();
            let now = now.with_nanosecond(0).unwrap();

            let delay = ReminderScheduler::get_target_delay(&fire_at, now);
            let target_datetime = now + delay;

            assert!(target_datetime > now, "Target time should always be in the future");
            assert!(target_datetime.time() == fire_at, "Target time should be equal to fire_at time specified in the reminder. fire_at = {:?}, target_datetime.time() = {:?}, target_datetime = {:?}", fire_at, target_datetime.time(), target_datetime);
            assert!(delay.num_days() <= 1, "Delay should be one day or less. delay.days = {}", delay.num_days())
        }
    }
}
