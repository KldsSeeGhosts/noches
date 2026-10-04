use rusqlite::{Connection, params};
use serde_json::{Value, json};
use zeron_proto::orchestration::ProjectId;

use crate::orchestration::{Error, Result};

pub(crate) fn search(conn: &Connection, project: &ProjectId, input: &Value) -> Result<Value> {
    let query = input["query"]
        .as_str()
        .unwrap_or("")
        .trim_matches(js_whitespace);
    if !(2..=200).contains(&query.encode_utf16().count()) {
        return Err(Error::Invariant("Thread search input is invalid.".into()));
    }
    let limit = input["limit"].as_i64().unwrap_or(50);
    if !(1..=50).contains(&limit) {
        return Err(Error::Invariant("Thread search input is invalid.".into()));
    }
    let pattern = format!(
        "%{}%",
        query
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_")
    );
    // Deliberately match T3's implementation, not the tool's title-search prose:
    // finished user/assistant content only; global top-N then project filtering.
    let mut statement = conn.prepare(
        "WITH candidate AS (
            SELECT t.id thread_id, json_extract(t.payload_json,'$.projectId') project_id,
              json_extract(m.payload_json,'$.role') source,
              json_extract(m.payload_json,'$.text') match_text,
              json_extract(m.payload_json,'$.createdAt') message_created_at,
              json_extract(t.payload_json,'$.updatedAt') thread_updated_at,
              CASE json_extract(m.payload_json,'$.role') WHEN 'user' THEN 0 ELSE 1 END match_rank,
              ROW_NUMBER() OVER (PARTITION BY t.id ORDER BY
                CASE json_extract(m.payload_json,'$.role') WHEN 'user' THEN 0 ELSE 1 END,
                json_extract(m.payload_json,'$.createdAt') DESC,m.id ASC) thread_match_rank
            FROM orchestration_projection_records m
            JOIN orchestration_projection_threads t ON t.id=m.thread_id
            WHERE m.kind='message'
              AND json_extract(t.payload_json,'$.deletedAt') IS NULL
              AND json_extract(t.payload_json,'$.archivedAt') IS NULL
              AND json_extract(m.payload_json,'$.streaming')=0
              AND json_extract(m.payload_json,'$.role') IN ('user','assistant')
              AND json_extract(m.payload_json,'$.text') LIKE ?1 ESCAPE '!')
         SELECT thread_id,project_id,source,match_text,message_created_at
         FROM candidate WHERE thread_match_rank=1
         ORDER BY match_rank,thread_updated_at DESC,thread_id LIMIT ?2",
    )?;
    let rows = statement.query_map(params![pattern, limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    let mut matches = vec![];
    for row in rows {
        let (id, project_id, source, text, created_at) = row?;
        if project_id != project.0 {
            continue;
        }
        matches.push(json!({"threadId":id,"projectId":project_id,"source":source,
            "snippet":snippet(&text,query),"messageCreatedAt":created_at}));
    }
    Ok(json!({"matches":matches}))
}

fn snippet(text: &str, query: &str) -> Value {
    let text = text
        .split(js_whitespace)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let units: Vec<_> = text.encode_utf16().collect();
    if units.len() <= 240 {
        return json!(text);
    }
    let normalized = text.to_ascii_lowercase();
    let query = query
        .split(js_whitespace)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let index = normalized
        .find(&query)
        .map(|offset| normalized[..offset].encode_utf16().count())
        .unwrap_or(0);
    let start = index.saturating_sub(72).min(units.len() - 236);
    let end = (start + 236).min(units.len());
    let mut snippet = vec![];
    if start > 0 {
        snippet.push('…' as u16);
    }
    snippet.extend_from_slice(&units[start..end]);
    if end < units.len() {
        snippet.push('…' as u16);
    }
    crate::orchestration::threads::wire::text_value(&snippet)
}

pub(crate) fn js_whitespace(c: char) -> bool {
    matches!(c,'\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{00A0}' | '\u{1680}' |
        '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' |
        '\u{205F}' | '\u{3000}' | '\u{FEFF}')
}

#[cfg(test)]
mod tests {
    #[test]
    fn snippets_are_bounded_and_centered() {
        let text = format!("{}needle{}", "x".repeat(300), "y".repeat(400));
        let value = super::snippet(&text, "needle");
        let snippet = value.as_str().unwrap();
        assert_eq!(snippet.encode_utf16().count(), 238);
        assert!(snippet.contains("needle"));
        assert_eq!(super::snippet("a \n b", "a"), "a b");
    }

    #[test]
    fn search_snippets_preserve_surrogate_boundaries_over_mcp() {
        use crate::orchestration::threads::wire;
        // Match is at UTF-16 index 73, so start=1 and end=237. Place a
        // surrogate pair across the end boundary to exercise JS slice.
        let text = format!(
            "{}needle{}😀{}",
            "x".repeat(73),
            "y".repeat(157),
            "y".repeat(400)
        );
        let snippet = super::snippet(&text, "needle");
        let page = serde_json::json!({"matches":[{"snippet":snippet}]});
        let encoded = wire::read_json(&page);
        assert!(encoded.contains(r"\ud83d"));
        assert!(!encoded.contains("noches.thread.utf16"));
        let response = serde_json::json!({"result":wire::result(page)});
        let encoded = wire::response_json(&response);
        assert!(encoded.contains(r"\ud83d"));
        assert!(!encoded.contains("noches.thread.utf16"));
    }
}
