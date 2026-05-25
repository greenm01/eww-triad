use serde_json::{Value, json};

use crate::error::{Error, Result};

pub const TRIAD_IPC_VERSION: u64 = 1;

pub fn request(name: &str) -> Value {
    json!({"triad": {"version": TRIAD_IPC_VERSION, "request": name}})
}

pub fn event_stream_request() -> Value {
    json!({
        "triad": {
            "version": TRIAD_IPC_VERSION,
            "request": "event-stream",
            "events": ["state", "layout", "window"]
        }
    })
}

pub fn action_request(action: &str, payload: Value) -> Result<Value> {
    let mut payload = match payload {
        Value::Object(map) => map,
        other => return Err(Error::InvalidActionPayload(other.to_string())),
    };
    payload.insert("version".to_string(), json!(TRIAD_IPC_VERSION));
    payload.insert("request".to_string(), json!("action"));
    payload.insert("action".to_string(), json!(action));
    Ok(json!({"triad": Value::Object(payload)}))
}

pub fn set_layout_request(layout: &str, tag: Option<u64>, workspace_idx: Option<u64>) -> Value {
    let mut payload = serde_json::Map::new();
    payload.insert("version".to_string(), json!(TRIAD_IPC_VERSION));
    payload.insert("request".to_string(), json!("set-layout"));
    payload.insert("layout".to_string(), json!(layout));
    if let Some(tag) = tag {
        payload.insert("target".to_string(), json!({"tag": tag}));
    } else if let Some(workspace_idx) = workspace_idx {
        payload.insert(
            "target".to_string(),
            json!({"workspace_idx": workspace_idx}),
        );
    }
    json!({"triad": Value::Object(payload)})
}

pub fn reply_ok(value: &Value) -> Result<&Value> {
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown triad error")
            .to_string();
        return Err(Error::Triad(message));
    }
    value
        .get("triad")
        .ok_or_else(|| Error::Triad("missing triad reply envelope".to_string()))
}

pub fn reply_type(reply: &Value) -> Option<&str> {
    reply.get("type").and_then(Value::as_str)
}

pub fn state_from_reply(value: &Value) -> Result<Value> {
    let triad = reply_ok(value)?;
    let actual = reply_type(triad).unwrap_or("<missing>");
    if actual != "state" {
        return Err(Error::UnexpectedReply {
            expected: "state".to_string(),
            actual: actual.to_string(),
        });
    }
    triad
        .get("state")
        .cloned()
        .ok_or_else(|| Error::Triad("state reply did not include state".to_string()))
}

pub fn event_name(value: &Value) -> Option<&str> {
    value
        .get("triad")
        .and_then(|triad| triad.get("event"))
        .and_then(Value::as_str)
}

pub fn event_payload(value: &Value) -> Option<&Value> {
    value.get("triad")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn action_request_merges_payload_fields() {
        let value = action_request("focus-workspace", json!({"workspace_idx": 2})).unwrap();
        assert_eq!(value["triad"]["version"], json!(1));
        assert_eq!(value["triad"]["request"], json!("action"));
        assert_eq!(value["triad"]["action"], json!("focus-workspace"));
        assert_eq!(value["triad"]["workspace_idx"], json!(2));
    }

    #[test]
    fn set_layout_can_target_workspace() {
        let value = set_layout_request("scroller", None, Some(3));
        assert_eq!(value["triad"]["request"], json!("set-layout"));
        assert_eq!(value["triad"]["target"]["workspace_idx"], json!(3));
    }

    #[test]
    fn state_reply_rejects_error_envelope() {
        let err = state_from_reply(&json!({"ok": false, "error": "bad"})).unwrap_err();
        assert!(err.to_string().contains("bad"));
    }
}
