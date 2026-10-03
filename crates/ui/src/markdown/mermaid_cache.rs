//! Lazily rendered Mermaid diagrams for a surface that discovers its fences
//! while painting (the agent transcript).
//!
//! Rows request their fences as they lay out, so only painted diagrams cost
//! anything. The owner renders one requested source at a time on a background
//! executor and drops requests whose rows left the viewport before their turn.
//! Results are retained under a byte budget, least recently painted first out;
//! an evicted diagram simply renders again when its row returns. Diagrams
//! painted in the latest two passes are never evicted. If those fill the
//! budget, additional fences keep their source.
use crate::image_media::MediaImage;
use gpui::SharedString;
use std::collections::{HashMap, HashSet};

pub(crate) const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 64;
const MAX_ROWS_PER_SOURCE: usize = 128;

pub(crate) enum Lookup {
    Pending,
    Ready(MediaImage),
    Failed(SharedString),
}

enum State {
    Pending,
    Ready(MediaImage),
    Failed(SharedString),
}

struct Entry {
    state: State,
    /// Paint pass that last requested this source.
    used: u64,
    /// Rows painting this source in its latest pass, remeasured on completion.
    rows: Vec<SharedString>,
}

#[derive(Default)]
pub(crate) struct MermaidCache {
    entries: HashMap<String, Entry>,
    frame: u64,
    style: Option<u32>,
    view: Option<((f32, f32), f32)>,
    new_requests: bool,
    /// Diagram frames switched to their source, keyed by frame id.
    source_visible: HashSet<SharedString>,
}

/// The renderer keys a diagram frame `"{row_key}-mermaid-{ix}"`.
pub(crate) fn frame_row(frame_id: &str) -> &str {
    frame_id
        .rsplit_once("-mermaid-")
        .map_or(frame_id, |(row, _)| row)
}

impl MermaidCache {
    /// Start a paint pass. A theme change discards every diagram, since the
    /// palette is baked into the generated SVG; the media is returned for
    /// release.
    pub(crate) fn begin_frame(&mut self, style: u32) -> Vec<MediaImage> {
        self.frame += 1;
        if self.style == Some(style) {
            return self.evict(1, 0);
        }
        self.style = Some(style);
        self.drain()
    }

    /// Rasterize retained diagrams for the column width and display density.
    /// Returns superseded rasters for release.
    pub(crate) fn set_view(&mut self, target: (f32, f32), scale: f32) -> Vec<MediaImage> {
        if self.view == Some((target, scale)) {
            return Vec::new();
        }
        self.view = Some((target, scale));
        let mut retired = Vec::new();
        let mut used = self.retained_bytes();
        for entry in self.entries.values_mut() {
            if let State::Ready(media) = &mut entry.state {
                let others = used - media.bytes;
                let next =
                    media.preview_within(target, scale, MAX_RETAINED_BYTES.saturating_sub(others));
                if !std::sync::Arc::ptr_eq(&media.image, &next.image) {
                    used = others + next.bytes;
                    retired.push(std::mem::replace(media, next));
                }
            }
        }
        retired
    }

    /// Look up a fence painted in the current pass, queueing it if unseen.
    pub(crate) fn request(&mut self, code: &str, frame_id: &str) -> Lookup {
        if let Err(reason) = super::mermaid::validate_source(code) {
            return Lookup::Failed(reason.into());
        }
        let frame = self.frame;
        if !self.entries.contains_key(code) && self.entries.len() >= MAX_ENTRIES {
            // Retire a stale failed or pending entry without dropping painted
            // media here, where there is no App to release its GPU assets.
            let stale = self
                .entries
                .iter()
                .filter(|(_, entry)| {
                    !matches!(entry.state, State::Ready(_)) && frame.saturating_sub(entry.used) > 1
                })
                .min_by_key(|(_, entry)| entry.used)
                .map(|(code, _)| code.clone());
            if let Some(stale) = stale {
                self.entries.remove(&stale);
            } else {
                return Lookup::Failed("Diagram cache limit reached".into());
            }
        }
        let entry = match self.entries.get_mut(code) {
            Some(entry) => entry,
            None => {
                self.new_requests = true;
                self.entries.entry(code.to_owned()).or_insert(Entry {
                    state: State::Pending,
                    used: frame,
                    rows: Vec::new(),
                })
            }
        };
        if entry.used != frame {
            entry.rows.clear();
        }
        entry.used = frame;
        let row = frame_row(frame_id);
        if !entry.rows.iter().any(|known| known == row) {
            if entry.rows.len() >= MAX_ROWS_PER_SOURCE {
                return Lookup::Failed("Diagram cache row limit reached".into());
            }
            entry.rows.push(row.to_owned().into());
        }
        match &entry.state {
            State::Pending => Lookup::Pending,
            State::Ready(media) => Lookup::Ready(media.clone()),
            State::Failed(reason) => Lookup::Failed(reason.clone()),
        }
    }

    /// Whether a fence was requested since the last call.
    pub(crate) fn take_new_requests(&mut self) -> bool {
        std::mem::take(&mut self.new_requests)
    }

    /// The next source to render. Requests not repainted in the latest two
    /// passes belong to rows that scrolled away; they are forgotten and
    /// requested again if their row returns. Two passes, because a request
    /// can be made while the current pass has only laid out some of its rows.
    pub(crate) fn next_job(&mut self) -> Option<String> {
        let frame = self.frame;
        self.entries.retain(|_, entry| {
            !matches!(entry.state, State::Pending) || frame.saturating_sub(entry.used) <= 1
        });
        self.entries
            .iter()
            .filter(|(_, entry)| matches!(entry.state, State::Pending))
            .max_by_key(|(_, entry)| entry.used)
            .map(|(code, _)| code.clone())
    }

    /// Store a finished render. Returns the rows that painted it (to
    /// remeasure) and evicted media (to release); `None` when the result is
    /// obsolete because the theme changed while it rendered.
    pub(crate) fn finish(
        &mut self,
        code: String,
        style: u32,
        result: Result<MediaImage, String>,
    ) -> Option<(Vec<SharedString>, Vec<MediaImage>)> {
        if self.style != Some(style) {
            return None;
        }
        // An in-flight source may have scrolled away and been pruned. Do not
        // resurrect it or bypass the entry admission limit.
        if !self.entries.contains_key(&code) {
            return Some((Vec::new(), result.into_iter().collect()));
        }
        let mut released = Vec::new();
        let state = match result {
            Ok(media) => {
                released.extend(self.evict(0, media.bytes));
                let available = MAX_RETAINED_BYTES.saturating_sub(self.retained_bytes());
                let media = match self.view {
                    Some((target, scale)) => media.preview_within(target, scale, available),
                    None => media,
                };
                if media.bytes > available {
                    released.push(media);
                    State::Failed("Diagram cache memory limit reached".into())
                } else {
                    State::Ready(media)
                }
            }
            Err(reason) => State::Failed(reason.into()),
        };
        let entry = self.entries.get_mut(&code).expect("requested diagram");
        debug_assert!(matches!(entry.state, State::Pending));
        entry.state = state;
        let rows = entry.rows.clone();
        released.extend(self.evict(0, 0));
        Some((rows, released))
    }

    /// Release settled diagrams least recently painted first until the
    /// retained memory and entry count fit their limits.
    fn evict(&mut self, reserve: usize, incoming_bytes: usize) -> Vec<MediaImage> {
        let mut released = Vec::new();
        let frame = self.frame;
        while self.retained_bytes().saturating_add(incoming_bytes) > MAX_RETAINED_BYTES
            || self.entries.len() > MAX_ENTRIES - reserve
        {
            let Some(code) = self
                .entries
                .iter()
                .filter(|(_, entry)| {
                    !matches!(entry.state, State::Pending) && frame.saturating_sub(entry.used) > 1
                })
                .min_by_key(|(_, entry)| entry.used)
                .map(|(code, _)| code.clone())
            else {
                break;
            };
            if let Some(Entry {
                state: State::Ready(media),
                ..
            }) = self.entries.remove(&code)
            {
                released.push(media);
            }
        }
        released
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.entries
            .values()
            .filter_map(|entry| match &entry.state {
                State::Ready(media) => Some(media.bytes),
                _ => None,
            })
            .sum()
    }

    /// Forget every diagram, returning retained media for release.
    pub(crate) fn drain(&mut self) -> Vec<MediaImage> {
        self.entries
            .drain()
            .filter_map(|(_, entry)| match entry.state {
                State::Ready(media) => Some(media),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn source_visible(&self, frame_id: &str) -> bool {
        self.source_visible.contains(frame_id)
    }

    pub(crate) fn toggle_source(&mut self, frame_id: &SharedString) {
        if !self.source_visible.remove(frame_id) {
            self.source_visible.insert(frame_id.clone());
        }
    }

    // Used by the transcript tests, which only run on Linux.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn ready_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| matches!(entry.state, State::Ready(_)))
            .count()
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn ready_media(&self) -> Option<MediaImage> {
        self.entries.values().find_map(|entry| match &entry.state {
            State::Ready(media) => Some(media.clone()),
            _ => None,
        })
    }

    pub(crate) fn has_source_toggles(&self) -> bool {
        !self.source_visible.is_empty()
    }

    /// Keep source toggles only for frames whose row still exists.
    pub(crate) fn retain_rows(&mut self, rows: &HashSet<&str>) {
        self.source_visible
            .retain(|frame_id| rows.contains(frame_row(frame_id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(width: u32) -> MediaImage {
        crate::image_media::decode_image(
            "image/svg+xml",
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="40"><rect width="{width}" height="40"/></svg>"#
            )
            .into_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn requests_queue_once_and_track_their_rows() {
        let mut cache = MermaidCache::default();
        assert!(cache.begin_frame(1).is_empty());
        assert!(matches!(
            cache.request("graph TD; A-->B", "chat#p.0-mermaid-3"),
            Lookup::Pending
        ));
        assert!(matches!(
            cache.request("graph TD; A-->B", "chat#p.2-mermaid-0"),
            Lookup::Pending
        ));
        assert!(cache.take_new_requests());
        assert!(!cache.take_new_requests());
        let code = cache.next_job().unwrap();
        let (rows, released) = cache.finish(code, 1, Ok(media(80))).unwrap();
        assert_eq!(rows, ["chat#p.0", "chat#p.2"]);
        assert!(released.is_empty());
        assert!(matches!(
            cache.request("graph TD; A-->B", "chat#p.0-mermaid-3"),
            Lookup::Ready(_)
        ));
        assert!(!cache.take_new_requests());
        assert!(cache.next_job().is_none());
    }

    #[test]
    fn requests_that_scrolled_away_are_dropped_before_rendering() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("a", "row-mermaid-0");
        cache.begin_frame(1);
        cache.request("b", "row2-mermaid-0");
        // `a` was painted in the previous pass: still eligible.
        assert!(cache.next_job().is_some());
        cache.begin_frame(1);
        cache.request("b", "row2-mermaid-0");
        assert_eq!(cache.next_job().as_deref(), Some("b"));
        cache.finish("b".into(), 1, Err("bad".into()));
        assert!(cache.next_job().is_none());
        assert!(matches!(
            cache.request("b", "row2-mermaid-0"),
            Lookup::Failed(_)
        ));
    }

    #[test]
    fn theme_changes_discard_diagrams_and_obsolete_results() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("a", "row-mermaid-0");
        cache.finish("a".into(), 1, Ok(media(80)));
        assert_eq!(cache.begin_frame(2).len(), 1);
        assert!(matches!(
            cache.request("a", "row-mermaid-0"),
            Lookup::Pending
        ));
        assert!(cache.finish("a".into(), 1, Ok(media(80))).is_none());
        assert!(matches!(
            cache.request("a", "row-mermaid-0"),
            Lookup::Pending
        ));
    }

    #[test]
    fn eviction_spares_diagrams_painted_in_the_latest_passes() {
        let mut cache = MermaidCache::default();
        for ix in 0..MAX_ENTRIES {
            cache.begin_frame(1);
            let code = format!("graph {ix}");
            cache.request(&code, &format!("row{ix}-mermaid-0"));
            cache.finish(code, 1, Err("bad".into()));
        }
        cache.request("graph 0", "row0-mermaid-0");
        cache.begin_frame(1);
        cache.request("graph 0", "row0-mermaid-0");
        cache.begin_frame(1);
        cache.request("graph 0", "row0-mermaid-0");
        cache.request("fresh", "fresh-mermaid-0");
        cache.finish("fresh".into(), 1, Ok(media(80)));
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert!(cache.entries.contains_key("graph 0"));
        assert!(cache.entries.contains_key("fresh"));
        assert!(!cache.entries.contains_key("graph 1"));
    }

    #[test]
    fn source_toggles_follow_their_rows() {
        let mut cache = MermaidCache::default();
        let frame: SharedString = "chat#p.0-mermaid-2".into();
        cache.toggle_source(&frame);
        assert!(cache.source_visible(&frame));
        cache.retain_rows(&HashSet::from(["chat#p.0"]));
        assert!(cache.source_visible(&frame));
        cache.retain_rows(&HashSet::from(["chat#p.1"]));
        assert!(!cache.source_visible(&frame));
    }

    #[test]
    fn oversized_source_is_not_retained_or_queued() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        let source = "x".repeat(super::super::mermaid::MAX_SOURCE_BYTES + 1);
        assert!(matches!(
            cache.request(&source, "row-mermaid-0"),
            Lookup::Failed(_)
        ));
        assert!(cache.entries.is_empty());
        assert!(!cache.take_new_requests());
        assert!(cache.next_job().is_none());
    }

    #[test]
    fn painted_and_pending_entries_cannot_exceed_the_hard_cap() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        for ix in 0..MAX_ENTRIES {
            assert!(matches!(
                cache.request(&format!("graph {ix}"), &format!("row{ix}-mermaid-0")),
                Lookup::Pending
            ));
        }
        assert!(matches!(
            cache.request("overflow", "extra-mermaid-0"),
            Lookup::Failed(_)
        ));
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        cache.begin_frame(1);
        cache.begin_frame(1);
        assert!(matches!(
            cache.request("new", "new-mermaid-0"),
            Lookup::Pending
        ));
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
    }

    #[test]
    fn oversized_result_fails_soft_without_exceeding_the_memory_budget() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("a", "row-mermaid-0");
        let mut oversized = media(80);
        oversized.bytes = MAX_RETAINED_BYTES + 1;
        let (rows, released) = cache.finish("a".into(), 1, Ok(oversized)).unwrap();
        assert_eq!(rows, ["row"]);
        assert_eq!(released.len(), 1);
        assert_eq!(cache.retained_bytes(), 0);
        assert!(matches!(
            cache.request("a", "row-mermaid-0"),
            Lookup::Failed(_)
        ));
    }

    #[test]
    fn late_results_do_not_resurrect_scrolled_away_requests() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("a", "row-mermaid-0");
        cache.begin_frame(1);
        cache.begin_frame(1);
        assert!(cache.next_job().is_none());
        cache.finish("a".into(), 1, Ok(media(80)));
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn a_failed_render_stays_source_and_is_not_requeued() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("bad", "row-mermaid-0");
        cache.take_new_requests();
        cache.finish("bad".into(), 1, Err("Diagram render timed out".into()));
        match cache.request("bad", "row-mermaid-0") {
            Lookup::Failed(reason) => assert_eq!(reason.as_ref(), "Diagram render timed out"),
            _ => panic!("failed native work must keep source"),
        }
        assert!(!cache.take_new_requests());
        assert!(cache.next_job().is_none());
    }

    #[test]
    fn repeated_source_cannot_retain_unbounded_row_ids() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        for ix in 0..MAX_ROWS_PER_SOURCE {
            assert!(matches!(
                cache.request("a", &format!("row{ix}-mermaid-0")),
                Lookup::Pending
            ));
        }
        assert!(matches!(
            cache.request("a", "overflow-mermaid-0"),
            Lookup::Failed(_)
        ));
        assert_eq!(cache.entries["a"].rows.len(), MAX_ROWS_PER_SOURCE);
        cache.begin_frame(1);
        assert!(matches!(
            cache.request("a", "next-pass-mermaid-0"),
            Lookup::Pending
        ));
        assert_eq!(cache.entries["a"].rows, ["next-pass"]);
    }

    #[test]
    fn admission_releases_stale_media_to_make_room_for_newly_painted_diagrams() {
        let mut cache = MermaidCache::default();
        cache.begin_frame(1);
        cache.request("old", "old-mermaid-0");
        let mut full = media(80);
        full.bytes = MAX_RETAINED_BYTES;
        cache.finish("old".into(), 1, Ok(full));
        cache.begin_frame(1);
        cache.begin_frame(1);
        cache.request("new", "new-mermaid-0");
        let (_, released) = cache.finish("new".into(), 1, Ok(media(80))).unwrap();
        assert_eq!(released.len(), 1);
        assert!(!cache.entries.contains_key("old"));
        assert!(matches!(
            cache.request("new", "new-mermaid-0"),
            Lookup::Ready(_)
        ));
        assert!(cache.retained_bytes() < MAX_RETAINED_BYTES);
    }
}
