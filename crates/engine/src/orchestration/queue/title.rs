//! T3's bounded user-intent-first title context, with a text-only host adapter.
use serde_json::Value;
use std::collections::BTreeMap;

const OMITTED: &str = "[Earlier content truncated]\n\n";
const TRUNCATED: &str = "\n[Content truncated]\n";

fn length(text: &str) -> usize {
    text.encode_utf16().count()
}

fn limit(text: &str, budget: usize) -> String {
    let units: Vec<_> = text.encode_utf16().collect();
    if units.len() <= budget {
        return text.into();
    }
    if budget <= length(TRUNCATED) {
        return String::new();
    }
    let available = budget - length(TRUNCATED);
    let head = available.div_ceil(2);
    let tail = available - head;
    format!(
        "{}{TRUNCATED}{}",
        String::from_utf16_lossy(&units[..head]),
        String::from_utf16_lossy(&units[units.len() - tail..])
    )
}

pub(crate) fn context(messages: &[Value]) -> String {
    let mut messages: Vec<_> = messages
        .iter()
        .filter(|m| {
            m["streaming"] != true && matches!(m["role"].as_str(), Some("user" | "assistant"))
        })
        .collect();
    messages.sort_by(|a, b| {
        a["createdAt"]
            .as_str()
            .cmp(&b["createdAt"].as_str())
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    let sections: Vec<_> = messages
        .into_iter()
        .filter_map(|m| {
            let role = m["role"].as_str()?;
            let text = m["text"]
                .as_str()
                .unwrap_or("")
                .trim_matches(super::search::js_whitespace);
            let names = m["attachments"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|a| a["name"].as_str())
                .collect::<Vec<_>>()
                .join(", ");
            if text.is_empty() && names.is_empty() {
                return None;
            }
            Some((
                role,
                format!("{}:\n", role.to_ascii_uppercase()),
                format!(
                    "{text}{}",
                    if names.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "{}[Attachments: {names}]",
                            if text.is_empty() { "" } else { "\n" }
                        )
                    }
                ),
            ))
        })
        .collect();
    let mut remaining = 8000 - length(OMITTED);
    let mut selected = BTreeMap::new();
    let add = |index: usize,
               budget: usize,
               remaining: &mut usize,
               selected: &mut BTreeMap<usize, String>| {
        if selected.contains_key(&index) {
            return;
        }
        let (_, prefix, contents) = &sections[index];
        let budget = budget.min(*remaining).saturating_sub(length(prefix) + 2);
        if budget <= length(TRUNCATED) {
            return;
        }
        let contents = limit(contents, budget);
        if !contents.is_empty() {
            let text = format!("{prefix}{contents}");
            *remaining -= length(&text) + 2;
            selected.insert(index, text);
        }
    };
    if let Some(index) = sections.iter().position(|s| s.0 == "user") {
        add(index, 2000, &mut remaining, &mut selected);
    }
    for index in (0..sections.len()).rev() {
        if sections[index].0 == "user" {
            add(
                index,
                2000.min(remaining.saturating_sub(2000)),
                &mut remaining,
                &mut selected,
            );
        }
    }
    for index in (0..sections.len()).rev() {
        if sections[index].0 == "assistant" {
            add(index, 2000, &mut remaining, &mut selected);
        }
    }
    for role in ["user", "assistant"] {
        for index in (0..sections.len()).rev() {
            let Some(previous) = selected.get(&index) else {
                continue;
            };
            let (section_role, prefix, contents) = &sections[index];
            if *section_role != role {
                continue;
            }
            let expanded = format!(
                "{prefix}{}",
                limit(contents, length(previous) + remaining - length(prefix))
            );
            remaining = remaining + length(previous) - length(&expanded);
            selected.insert(index, expanded);
        }
    }
    let truncated = selected
        .iter()
        .any(|(index, value)| *value != format!("{}{}", sections[*index].1, sections[*index].2));
    format!(
        "{}{}",
        if truncated || selected.len() < sections.len() {
            OMITTED
        } else {
            ""
        },
        selected.into_values().collect::<Vec<_>>().join("\n\n")
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn title_context_is_finished_bounded_and_preserves_user_intent() {
        let messages = vec![
            json!({"role":"user","text":"first request","createdAt":"1","id":"first","streaming":false}),
            json!({"role":"assistant","text":"x".repeat(20000),"createdAt":"2","id":"reply","streaming":false}),
            json!({"role":"user","text":"final constraint","createdAt":"3","id":"last","streaming":false,
                "attachments":[{"name":"fixture.png"}]}),
            json!({"role":"assistant","text":"unfinished","createdAt":"4","streaming":true}),
        ];
        let text = super::context(&messages);
        assert!(super::length(&text) <= 8000);
        assert!(text.contains("first request") && text.contains("final constraint"));
        assert!(text.contains("[Attachments: fixture.png]"));
        assert!(!text.contains("unfinished"));
        assert!(text.starts_with(super::OMITTED));
        assert_eq!(super::context(&[]), "");
    }
}
