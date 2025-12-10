use super::*;

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::Timelike;
use nadoeda_models::reminder::ReminderFireTime;
use proptest_arbitrary_interop::arb;

#[test]
pub fn when_firing_time_is_yet_to_come_target_delay_should_be_less_than_day() {
    let now_utc = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
        NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    );
    let now = DateTime::from_naive_utc_and_offset(now_utc, Utc);
    let fire_at = NaiveTime::from_hms_opt(13, 0, 0).unwrap();

    let delay = get_target_delay(&fire_at, now);

    assert_eq!(
        delay.num_hours(),
        1,
        "With given constraints the delay should be 1 hour."
    );
}

#[test]
pub fn when_firing_time_is_passed_target_delay_should_be_next_day() {
    let now_utc = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2025, 05, 31).unwrap(),
        NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    );
    let now = DateTime::from_naive_utc_and_offset(now_utc, Utc);

    let fire_at = ReminderFireTime::new(NaiveTime::from_hms_opt(11, 0, 0).unwrap());
    let delay = get_target_delay(fire_at.time(), now);

    assert_eq!(
        delay.num_hours(),
        23,
        "With given constraints, the delay should be 23 hours"
    );
}

proptest::proptest! {
    #[test]
    fn test_target_delay(
        now_utc in arb::<NaiveDateTime>(),
        fire_at in arb::<NaiveTime>()
    ) {
        let fire_at = fire_at.with_nanosecond(0).unwrap();
        let now = DateTime::from_naive_utc_and_offset(now_utc.with_nanosecond(0).unwrap(), Utc);
        let delay = get_target_delay(&fire_at, now);
        let target_datetime = now + delay;

        assert!(target_datetime > now, "Target time should always be in the future");
        assert!(target_datetime.time() == fire_at, "Target time should be equal to fire_at time specified in the reminder. fire_at = {:?}, target_datetime.time() = {:?}, target_datetime = {:?}", fire_at, target_datetime.time(), target_datetime);
        assert!(delay.num_days() <= 1, "Delay should be one day or less. delay.days = {}", delay.num_days())
    }
}
