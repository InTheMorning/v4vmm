use rss::extension::{Extension, ExtensionMap};
use serde_json::{json, Value as JsonValue};

pub fn find_ext<'a>(exts: &'a ExtensionMap, ns: &str, name: &str) -> Option<&'a Extension> {
    exts.get(ns)?.get(name)?.first()
}

pub fn find_ext_text(exts: &ExtensionMap, ns: &str, name: &str) -> Option<String> {
    find_ext(exts, ns, name)?.value.clone()
}

pub fn find_ext_attr(exts: &ExtensionMap, ns: &str, name: &str, attr: &str) -> Option<String> {
    find_ext(exts, ns, name)?.attrs.get(attr).cloned()
}

// podcast:person -> JSON array [{ name, attrs }, ...]
pub fn collect_people_json(exts: &ExtensionMap) -> Option<String> {
    let persons = exts.get("podcast")?.get("person")?;
    let arr: Vec<JsonValue> = persons
        .iter()
        .map(|p| {
            json!({
                "name": p.value,
                "attrs": p.attrs,
            })
        })
        .collect();
    serde_json::to_string(&arr).ok()
}

pub fn ext_to_json(ext: &Extension) -> JsonValue {
    let children = ext
        .children
        .iter()
        .map(|(k, vec)| {
            let arr: Vec<JsonValue> = vec.iter().map(ext_to_json).collect();
            (k.clone(), JsonValue::Array(arr))
        })
        .collect::<serde_json::Map<String, JsonValue>>();

    json!({
        "value": ext.value,
        "attrs": ext.attrs,
        "children": children,
    })
}

pub fn value_block_json(
    exts: &rss::extension::ExtensionMap,
    ns: &str,
    name: &str,
) -> Option<String> {
    let ext = find_ext(exts, ns, name)?;
    serde_json::to_string(&ext_to_json(ext)).ok()
}

// "123", "03:45", "1:02:03" -> seconds
pub fn parse_itunes_duration(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let parts: Vec<&str> = raw.split(':').collect();
    let nums: Vec<i64> = parts
        .iter()
        .map(|p| p.parse::<i64>().ok())
        .collect::<Option<_>>()?;

    let secs = match nums.len() {
        1 => nums[0],
        2 => nums[0] * 60 + nums[1],
        3 => nums[0] * 3600 + nums[1] * 60 + nums[2],
        _ => return None,
    };
    Some(secs)
}
