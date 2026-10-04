//! Pin, snooze and settle from the sidebar (DESIGN-W3 §2, T3's thread
//! settlement). Presets and labels follow T3's `threadSettled.ts` so the same
//! wake time reads the same in both apps.

use chrono::{DateTime, Datelike as _, Duration, Local, TimeZone as _, Utc};
use zeron_proto::ChatLifecycle;
use zeron_rpc::methods;

use super::spaces::RowPark;
use super::*;

const EVENING_HOUR: u32 = 18;
const MORNING_HOUR: u32 = 9;

/// The `action` values of T3's `t3_thread_organize` that the sidebar sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OrganizeAction {
    Pin,
    Unpin,
    Snooze,
    Unsnooze,
    Settle,
    Unsettle,
}

impl OrganizeAction {
    fn wire(self) -> &'static str {
        match self {
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Snooze => "snooze",
            Self::Unsnooze => "unsnooze",
            Self::Settle => "settle",
            Self::Unsettle => "unsettle",
        }
    }

    /// The local, optimistic image of the action. The host's synced record
    /// replaces it when it lands.
    pub(super) fn apply(
        self,
        lifecycle: &mut ChatLifecycle,
        snoozed_until: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) {
        match self {
            Self::Pin => lifecycle.pinned_at = Some(now),
            Self::Unpin => lifecycle.pinned_at = None,
            Self::Snooze => {
                lifecycle.snoozed_until = snoozed_until;
                lifecycle.woke_at = None;
            }
            Self::Unsnooze => {
                lifecycle.snoozed_until = None;
                lifecycle.woke_at = None;
            }
            Self::Settle => {
                lifecycle.settled_at = Some(now);
                lifecycle.settled_by = Some(zeron_proto::SettleSource::User);
            }
            Self::Unsettle => {
                lifecycle.settled_at = None;
                lifecycle.settled_by = None;
            }
        }
    }
}

/// What the sidebar row draws for its lifecycle. Default for the palette,
/// the archive shelf and every chat with no lifecycle record.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct RowParking {
    pub pinned: bool,
    /// Parked in the Snoozed shelf until this instant.
    pub snoozed_until: Option<DateTime<Utc>>,
    /// Parked in the Settled shelf since this instant.
    pub settled_at: Option<DateTime<Utc>>,
    /// A snooze elapsed and the user has not acknowledged it yet.
    pub woke: bool,
}

impl RowParking {
    pub(super) fn of(park: RowPark, lifecycle: Option<&ChatLifecycle>) -> Self {
        let Some(lifecycle) = lifecycle else {
            return Self::default();
        };
        Self {
            pinned: lifecycle.pinned(),
            snoozed_until: (park == RowPark::Snoozed)
                .then_some(lifecycle.snoozed_until)
                .flatten(),
            settled_at: (park == RowPark::Settled)
                .then_some(lifecycle.settled_at)
                .flatten(),
            woke: lifecycle.woke(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SnoozePreset {
    pub label: &'static str,
    /// Complements the label instead of repeating it: "Tomorrow" pairs with
    /// "9:00 AM", not "tomorrow 9:00 AM".
    pub when: String,
    pub until: DateTime<Utc>,
}

fn time_of_day(at: DateTime<Local>) -> String {
    at.format("%-I:%M %p").to_string()
}

/// Local wall-clock `hour:00` on `date`'s calendar day plus `days`. Calendar
/// arithmetic, not fixed 24h offsets, so DST days land on the right morning.
fn at_hour(base: DateTime<Local>, days: i64, hour: u32) -> DateTime<Local> {
    let date = base.date_naive() + Duration::days(days);
    let naive = date.and_hms_opt(hour, 0, 0).expect("valid hour");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .unwrap_or_else(|| base + Duration::days(days))
}

/// T3's `resolveSnoozePresets`: 1h, 3h, this evening (only while more than an
/// hour away), tomorrow 9:00, and next Monday 9:00 unless that is tomorrow.
pub(super) fn snooze_presets(now: DateTime<Local>) -> Vec<SnoozePreset> {
    let in_hours = |hours: i64| now + Duration::hours(hours);
    let mut presets = vec![
        SnoozePreset {
            label: "In 1 hour",
            when: time_of_day(in_hours(1)),
            until: in_hours(1).with_timezone(&Utc),
        },
        SnoozePreset {
            label: "In 3 hours",
            when: time_of_day(in_hours(3)),
            until: in_hours(3).with_timezone(&Utc),
        },
    ];
    let evening = at_hour(now, 0, EVENING_HOUR);
    if evening - now > Duration::hours(1) {
        presets.push(SnoozePreset {
            label: "This evening",
            when: time_of_day(evening),
            until: evening.with_timezone(&Utc),
        });
    }
    let tomorrow = at_hour(now, 1, MORNING_HOUR);
    presets.push(SnoozePreset {
        label: "Tomorrow",
        when: time_of_day(tomorrow),
        until: tomorrow.with_timezone(&Utc),
    });
    let weekday = i64::from(now.weekday().num_days_from_monday());
    let days_until_monday = if weekday == 0 { 7 } else { 7 - weekday };
    let next_week = at_hour(now, days_until_monday, MORNING_HOUR);
    if next_week != tomorrow {
        presets.push(SnoozePreset {
            label: "Next week",
            when: format!("{} {}", next_week.format("%a"), time_of_day(next_week)),
            until: next_week.with_timezone(&Utc),
        });
    }
    presets
}

/// T3's compact "wakes in" label for a snoozed row's trailing slot: minutes
/// round up so a hidden row never reads "0m".
pub(super) fn wake_label(until: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let remaining = (until - now).num_milliseconds();
    const MINUTE: i64 = 60_000;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    let ceil = |unit: i64| (remaining + unit - 1) / unit;
    if remaining <= 0 {
        "now".into()
    } else if remaining < HOUR {
        format!("{}m", ceil(MINUTE).max(1))
    } else if remaining < DAY {
        format!("{}h", ceil(HOUR))
    } else {
        format!("{}d", ceil(DAY))
    }
}

impl Shell {
    /// Apply `action` optimistically and ask the chat's host to make it
    /// durable. A refusal surfaces as the sidebar notice; the synced record
    /// then restores the truth.
    pub(super) fn organize_chat(
        &mut self,
        chat_id: String,
        action: OrganizeAction,
        snoozed_until: Option<DateTime<Utc>>,
        cx: &mut Context<Self>,
    ) {
        self.close_chat_menu(cx);
        let now = Utc::now();
        let (engine, host) = {
            let state = self.state.read(cx);
            let host = state
                .chats
                .iter()
                .find(|chat| chat.id == chat_id)
                .map(|chat| chat.device_id.clone());
            (state.engine().cloned(), host)
        };
        let Some(engine) = engine else {
            self.sidebar_notice = Some("Engine not connected".into());
            cx.notify();
            return;
        };
        self.state.update(cx, |state, cx| {
            let lifecycle = state.thread_lifecycles.entry(chat_id.clone()).or_default();
            action.apply(lifecycle, snoozed_until, now);
            cx.notify();
        });
        self.sidebar_source_dirty.set(true);
        let mut params = serde_json::json!({ "chatId": chat_id, "action": action.wire() });
        if let Some(until) = snoozed_until {
            params["snoozedUntil"] = serde_json::Value::String(until.to_rfc3339());
        }
        if let Some(host) = host {
            params["targetDeviceId"] = serde_json::Value::String(host);
        }
        cx.spawn(async move |this, cx| {
            if let Err(err) = engine.client().call(methods::ORGANIZE_THREAD, params).await {
                this.update(cx, |shell, cx| {
                    shell.sidebar_notice = Some(format!("{err}").into());
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }

    pub(super) fn acknowledge_woke(&mut self, chat_id: String, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.state.update(cx, |state, cx| {
            if let Some(lifecycle) = state.thread_lifecycles.get_mut(&chat_id) {
                lifecycle.woke_at = None;
            }
            cx.notify();
        });
        self.sidebar_source_dirty.set(true);
        cx.spawn(async move |this, cx| {
            if let Err(err) = engine
                .client()
                .call(
                    methods::ACKNOWLEDGE_THREAD_WOKE,
                    serde_json::json!({ "chatId": chat_id }),
                )
                .await
            {
                this.update(cx, |shell, cx| {
                    shell.sidebar_notice = Some(format!("{err}").into());
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn presets_follow_t3() {
        // Wednesday afternoon: every preset is offered.
        let labels: Vec<_> = snooze_presets(local(2026, 10, 7, 14, 0))
            .into_iter()
            .map(|p| p.label)
            .collect();
        assert_eq!(
            labels,
            [
                "In 1 hour",
                "In 3 hours",
                "This evening",
                "Tomorrow",
                "Next week"
            ]
        );
        // 17:30 is within an hour of evening, so it drops.
        let late: Vec<_> = snooze_presets(local(2026, 10, 7, 17, 30))
            .into_iter()
            .map(|p| p.label)
            .collect();
        assert!(!late.contains(&"This evening"));
        // Sunday: tomorrow and next week are both Monday 9:00; only one stays.
        let sunday = snooze_presets(local(2026, 10, 4, 11, 0));
        assert!(sunday.iter().all(|p| p.label != "Next week"));
        let tomorrow = sunday.iter().find(|p| p.label == "Tomorrow").unwrap();
        assert_eq!(
            tomorrow.until.with_timezone(&Local),
            local(2026, 10, 5, 9, 0)
        );
        assert_eq!(tomorrow.when, "9:00 AM");
    }

    #[test]
    fn wake_label_rounds_up() {
        let now = Utc::now();
        assert_eq!(wake_label(now - Duration::seconds(1), now), "now");
        assert_eq!(wake_label(now + Duration::seconds(10), now), "1m");
        assert_eq!(wake_label(now + Duration::minutes(59), now), "59m");
        assert_eq!(wake_label(now + Duration::minutes(61), now), "2h");
        assert_eq!(wake_label(now + Duration::hours(30), now), "2d");
    }

    #[test]
    fn optimistic_apply_matches_organize_semantics() {
        let now = Utc::now();
        let mut lifecycle = ChatLifecycle {
            woke_at: Some(now),
            ..Default::default()
        };
        OrganizeAction::Snooze.apply(&mut lifecycle, Some(now + Duration::hours(1)), now);
        assert!(lifecycle.snoozed(now));
        assert!(!lifecycle.woke());
        OrganizeAction::Settle.apply(&mut lifecycle, None, now);
        OrganizeAction::Pin.apply(&mut lifecycle, None, now);
        assert!(lifecycle.settled() && lifecycle.pinned());
        OrganizeAction::Unsettle.apply(&mut lifecycle, None, now);
        OrganizeAction::Unpin.apply(&mut lifecycle, None, now);
        OrganizeAction::Unsnooze.apply(&mut lifecycle, None, now);
        assert!(lifecycle.is_default());
    }
}
