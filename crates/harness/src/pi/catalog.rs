//! Pi's model catalog: `get_available_models` entries become picker rows.
//!
//! Model ids are Pi's own `provider/id` slugs, so the composer can address any
//! model Pi has credentials for (including custom providers such as `cpa/...`).
//! The leading `default` row defers to the user's own Pi settings, which is
//! what Pi would do when launched bare.

use serde_json::Value;
use zeron_proto::{Model, ReasoningLevel};

/// Sentinel model id: do not select a model, Pi resolves its configured one.
pub(crate) const DEFAULT_MODEL: &str = "default";

/// Pi's thinking ladder in ascending order (`off` has no Noches equivalent
/// and is left to the model default).
const LADDER: [(&str, ReasoningLevel); 6] = [
    ("minimal", ReasoningLevel::Minimal),
    ("low", ReasoningLevel::Low),
    ("medium", ReasoningLevel::Medium),
    ("high", ReasoningLevel::High),
    ("xhigh", ReasoningLevel::XHigh),
    ("max", ReasoningLevel::Max),
];

pub(crate) fn level_name(level: ReasoningLevel) -> Option<&'static str> {
    LADDER
        .iter()
        .find_map(|(name, candidate)| (*candidate == level).then_some(*name))
}

/// The `Pi default` row: all levels, since the model it resolves to is only
/// known inside Pi. Pi clamps a level the resolved model lacks to its nearest.
pub(crate) fn default_model() -> Model {
    Model {
        id: DEFAULT_MODEL.into(),
        label: "Pi default".into(),
        description: Some("Runs the model configured in Pi (settings.json)".into()),
        reasoning_levels: LADDER.iter().map(|(_, level)| *level).collect(),
        options: Vec::new(),
    }
}

/// Mirrors pi-ai's `getSupportedThinkingLevels`: a reasoning model offers
/// minimal..high unless its map nulls a level out; xhigh and max appear only
/// when the map names them.
pub(crate) fn supported_levels(model: &Value) -> Vec<ReasoningLevel> {
    if model.get("reasoning").and_then(Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let map = model.get("thinkingLevelMap").and_then(Value::as_object);
    LADDER
        .iter()
        .filter(|(name, _)| {
            let entry = map.and_then(|map| map.get(*name));
            match entry {
                Some(Value::Null) => false,
                None => !matches!(*name, "xhigh" | "max"),
                Some(_) => true,
            }
        })
        .map(|(_, level)| *level)
        .collect()
}

fn describe(model: &Value) -> Option<String> {
    let provider = model.get("provider").and_then(Value::as_str)?;
    Some(match context_window(model) {
        Some(window) => format!("{provider} · {} context", compact_tokens(window)),
        None => provider.to_owned(),
    })
}

fn compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let millions = tokens as f64 / 1_000_000.0;
        let rounded = (millions * 10.0).round() / 10.0;
        if rounded.fract() == 0.0 {
            format!("{}M", rounded as u64)
        } else {
            format!("{rounded:.1}M")
        }
    } else {
        format!("{}K", tokens / 1000)
    }
}

pub(crate) fn context_window(model: &Value) -> Option<u64> {
    model
        .get("contextWindow")
        .and_then(Value::as_u64)
        .filter(|window| *window > 0)
}

/// Pi's slug for a model object (`provider/id`).
pub(crate) fn slug(model: &Value) -> Option<String> {
    let provider = model.get("provider").and_then(Value::as_str)?;
    let id = model.get("id").and_then(Value::as_str)?;
    (!provider.is_empty() && !id.is_empty()).then(|| format!("{provider}/{id}"))
}

/// Split a `provider/id` slug for `set_model`. The id may itself contain
/// slashes (`cpa/devin/swe-2`); only the first separates the provider.
pub(crate) fn split_slug(slug: &str) -> Option<(&str, &str)> {
    let (provider, id) = slug.split_once('/')?;
    (!provider.is_empty() && !id.is_empty()).then_some((provider, id))
}

/// Picker rows plus each model's declared context window, from a
/// `get_available_models` payload. Duplicate slugs keep their first entry.
pub(crate) fn parse_models(data: &Value) -> (Vec<Model>, Vec<(String, u64)>) {
    let mut seen = std::collections::HashSet::new();
    let mut models = vec![default_model()];
    let mut windows = Vec::new();
    for entry in data
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(slug) = slug(entry) else { continue };
        if !seen.insert(slug.clone()) {
            continue;
        }
        if let Some(window) = context_window(entry) {
            windows.push((slug.clone(), window));
        }
        models.push(Model {
            label: entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .unwrap_or(&slug)
                .to_owned(),
            description: describe(entry),
            reasoning_levels: supported_levels(entry),
            options: Vec::new(),
            id: slug,
        });
    }
    (models, windows)
}

/// Mirrors pi-ai's `clampThinkingLevel`: the requested level when offered,
/// else the nearest higher level, else the nearest lower one.
pub(crate) fn clamp_level(
    requested: ReasoningLevel,
    available: &[ReasoningLevel],
) -> Option<ReasoningLevel> {
    if available.contains(&requested) {
        return Some(requested);
    }
    let order = |level: ReasoningLevel| LADDER.iter().position(|(_, l)| *l == level);
    let wanted = order(requested)?;
    let mut ranked: Vec<(usize, ReasoningLevel)> = available
        .iter()
        .filter_map(|level| order(*level).map(|index| (index, *level)))
        .collect();
    ranked.sort_by_key(|(index, _)| *index);
    ranked
        .iter()
        .find(|(index, _)| *index > wanted)
        .or_else(|| ranked.iter().rev().find(|(index, _)| *index < wanted))
        .map(|(_, level)| *level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn thinking_ladder_follows_the_models_level_map() {
        // Observed live (Pi 1.0.4, cpa/gemini-3.8-flash): only low..high.
        let model = json!({"reasoning":true,"thinkingLevelMap":{
            "high":"high","low":"low","max":null,"medium":"medium",
            "minimal":null,"off":null,"xhigh":null}});
        assert_eq!(
            supported_levels(&model),
            vec![
                ReasoningLevel::Low,
                ReasoningLevel::Medium,
                ReasoningLevel::High
            ]
        );
        // No map: minimal..high; xhigh/max only appear when named.
        assert_eq!(
            supported_levels(&json!({"reasoning":true})),
            vec![
                ReasoningLevel::Minimal,
                ReasoningLevel::Low,
                ReasoningLevel::Medium,
                ReasoningLevel::High
            ]
        );
        let full = json!({"reasoning":true,"thinkingLevelMap":{"xhigh":"xhigh","max":"max"}});
        assert_eq!(supported_levels(&full).len(), 6);
        assert!(supported_levels(&json!({"reasoning":false})).is_empty());
        assert!(supported_levels(&json!({})).is_empty());
    }

    #[test]
    fn catalog_lists_exact_provider_slugs_after_the_default_row() {
        let data = json!({"models":[
            {"id":"gemini-3.8-flash","name":"Gemini 3.8 Flash","provider":"cpa",
             "reasoning":true,"contextWindow":1048576},
            {"id":"devin/swe-2","provider":"cpa","reasoning":false,"contextWindow":262144},
            {"id":"gemini-3.8-flash","name":"dupe","provider":"cpa"},
            {"id":"","provider":"cpa"},
            {"name":"no ids"}
        ]});
        let (models, windows) = parse_models(&data);
        let ids: Vec<_> = models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["default", "cpa/gemini-3.8-flash", "cpa/devin/swe-2"]);
        assert_eq!(models[1].label, "Gemini 3.8 Flash");
        assert_eq!(models[1].description.as_deref(), Some("cpa · 1M context"));
        assert_eq!(models[2].label, "cpa/devin/swe-2");
        assert_eq!(models[2].description.as_deref(), Some("cpa · 262K context"));
        assert!(models[2].reasoning_levels.is_empty());
        assert_eq!(
            windows,
            vec![
                ("cpa/gemini-3.8-flash".to_owned(), 1_048_576),
                ("cpa/devin/swe-2".to_owned(), 262_144)
            ]
        );
    }

    #[test]
    fn slugs_split_on_the_first_separator_only() {
        assert_eq!(
            split_slug("cpa/devin/swe-2"),
            Some(("cpa", "devin/swe-2"))
        );
        assert_eq!(split_slug("default"), None);
        assert_eq!(split_slug("/id"), None);
        assert_eq!(split_slug("provider/"), None);
    }

    #[test]
    fn clamping_prefers_the_nearest_higher_level_like_pi() {
        use ReasoningLevel::*;
        let offered = [Low, Medium, High];
        assert_eq!(clamp_level(XHigh, &offered), Some(High));
        assert_eq!(clamp_level(Minimal, &offered), Some(Low));
        assert_eq!(clamp_level(Medium, &offered), Some(Medium));
        assert_eq!(clamp_level(Max, &[]), None);
        assert_eq!(clamp_level(Low, &[XHigh]), Some(XHigh));
    }
}
