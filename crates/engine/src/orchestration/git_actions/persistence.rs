use rusqlite::{OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::orchestration::Store;

pub fn get<T: DeserializeOwned>(store: &Store, ns: &str, key: &str) -> anyhow::Result<Option<T>> {
    Ok(store.read(|conn| {
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM git_actions_kv WHERE namespace=?1 AND key=?2",
                params![ns, key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(value.map(|v| serde_json::from_str(&v)).transpose()?)
    })?)
}

pub fn put<T: Serialize>(store: &Store, ns: &str, key: &str, value: &T) -> anyhow::Result<()> {
    let value = serde_json::to_string(value)?;
    store.write(|conn| {
        conn.execute(
            "INSERT INTO git_actions_kv VALUES(?1,?2,?3)
             ON CONFLICT(namespace,key) DO UPDATE SET value=excluded.value",
            params![ns, key, value],
        )?;
        Ok(())
    })?;
    Ok(())
}

pub fn list<T: DeserializeOwned>(store: &Store, ns: &str) -> anyhow::Result<Vec<T>> {
    Ok(store.read(|conn| {
        let mut stmt =
            conn.prepare("SELECT value FROM git_actions_kv WHERE namespace=?1 ORDER BY key")?;
        let values = stmt.query_map([ns], |r| r.get::<_, String>(0))?;
        let mut result = Vec::new();
        for value in values {
            result.push(serde_json::from_str(&value?)?);
        }
        Ok(result)
    })?)
}
