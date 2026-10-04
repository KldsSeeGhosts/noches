//! T3 Schedule.ts semantics. Wall-clock arithmetic uses the host's local zone.
use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, Offset, TimeZone, Utc};
use zeron_proto::orchestration::{Optional, ScheduledTaskSchedule};

pub const MIN_INTERVAL_MS: i64 = 60_000;
pub const FIXED_GRACE_MS: i64 = 600_000;

/// Fake clocks override this and `next_run_at` together, without global TZ or
/// process-clock mutation. Production resolves the local zone on each call.
pub trait Clock: Send + Sync + 'static {
    fn now_ms(&self) -> i64;
    fn next_run_at(&self, schedule: &ScheduledTaskSchedule, from_ms: i64) -> Option<i64> {
        next_in_zone(schedule, from_ms, &Local)
    }
}

pub struct HostClock;
impl Clock for HostClock {
    fn now_ms(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
}

pub fn parse_time(value: &str) -> Option<(u32, u32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    if !(1..=2).contains(&hour.len())
        || minute.len() != 2
        || !hour
            .bytes()
            .chain(minute.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let (hour, minute) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour < 24 && minute < 60).then_some((hour, minute))
}

/// Compatible wall-clock disambiguation: the earlier instant in a fall fold;
/// shift forward by the transition gap in spring (02:30 -> 03:30), not skip the
/// day or execute twice. Matches JS/Effect's compatible local-time adjustment.
fn resolve<T: TimeZone>(zone: &T, local: NaiveDateTime) -> Option<DateTime<T>> {
    match zone.from_local_datetime(&local) {
        LocalResult::Single(value) => Some(value),
        LocalResult::Ambiguous(a, b) => Some(a.min(b)),
        LocalResult::None => {
            // Also handles half-hour transitions and date-line jumps.
            let before = (1..=48).find_map(|hours| {
                zone.from_local_datetime(&local.checked_sub_signed(Duration::hours(hours))?)
                    .earliest()
            })?;
            let after = (1..=48).find_map(|hours| {
                zone.from_local_datetime(&local.checked_add_signed(Duration::hours(hours))?)
                    .earliest()
            })?;
            let gap =
                after.offset().fix().local_minus_utc() - before.offset().fix().local_minus_utc();
            if gap <= 0 {
                return None;
            }
            zone.from_local_datetime(&local.checked_add_signed(Duration::seconds(gap.into()))?)
                .earliest()
        }
    }
}

pub fn next_in_zone<T: TimeZone>(
    schedule: &ScheduledTaskSchedule,
    from_ms: i64,
    zone: &T,
) -> Option<i64> {
    match schedule {
        ScheduledTaskSchedule::Interval(value) => {
            let next = from_ms.checked_add(value.every_ms.max(MIN_INTERVAL_MS))?;
            DateTime::<Utc>::from_timestamp_millis(next).map(|_| next)
        }
        ScheduledTaskSchedule::FixedTime(value) => {
            use chrono::Datelike;
            let (hour, minute) = parse_time(&value.time_of_day)?;
            let from = DateTime::<Utc>::from_timestamp_millis(from_ms)?.with_timezone(zone);
            let weekdays = weekday_key(&value.weekdays);
            for offset in 0..=7 {
                let day = from
                    .date_naive()
                    .checked_add_signed(Duration::days(offset))?;
                let candidate = resolve(zone, day.and_hms_opt(hour, minute, 0)?)?;
                let weekday = i64::from(candidate.weekday().num_days_from_sunday());
                if candidate.timestamp_millis() > from_ms
                    && (weekdays.is_empty() || weekdays.contains(&weekday))
                {
                    return Some(candidate.timestamp_millis());
                }
            }
            None
        }
    }
}

fn weekday_key(value: &Optional<Vec<i64>>) -> BTreeSet<i64> {
    let mut days: BTreeSet<_> = match value {
        Optional::Present(value) => value.iter().copied().collect(),
        Optional::Absent => BTreeSet::new(),
    };
    if days.len() == 7 {
        days.clear();
    }
    days
}

pub fn same_schedule(a: &ScheduledTaskSchedule, b: &ScheduledTaskSchedule) -> bool {
    match (a, b) {
        (ScheduledTaskSchedule::Interval(a), ScheduledTaskSchedule::Interval(b)) => {
            a.every_ms == b.every_ms
        }
        (ScheduledTaskSchedule::FixedTime(a), ScheduledTaskSchedule::FixedTime(b)) => {
            parse_time(&a.time_of_day).is_some()
                && parse_time(&a.time_of_day) == parse_time(&b.time_of_day)
                && weekday_key(&a.weekdays) == weekday_key(&b.weekdays)
        }
        _ => false,
    }
}

pub fn missed_fixed(schedule: &ScheduledTaskSchedule, due_ms: i64, now_ms: i64) -> bool {
    matches!(schedule, ScheduledTaskSchedule::FixedTime(_))
        && now_ms.saturating_sub(due_ms) > FIXED_GRACE_MS
}

pub fn cadence(schedule: &ScheduledTaskSchedule) -> String {
    match schedule {
        ScheduledTaskSchedule::Interval(value) if value.every_ms % MIN_INTERVAL_MS == 0 => {
            let minutes = value.every_ms / MIN_INTERVAL_MS;
            if minutes == 1 {
                "Every minute".into()
            } else {
                format!("Every {minutes} minutes")
            }
        }
        ScheduledTaskSchedule::Interval(value) => {
            format!("Every {} seconds", (value.every_ms as f64 / 1000.0).round())
        }
        ScheduledTaskSchedule::FixedTime(value) => {
            let days = match &value.weekdays {
                Optional::Present(days)
                    if days.len() == 5 && days.iter().all(|day| (1..=5).contains(day)) =>
                {
                    "weekday"
                }
                Optional::Present(days) if !days.is_empty() => "selected day",
                _ => "day",
            };
            format!("At {} every {days}", value.time_of_day)
        }
    }
}
