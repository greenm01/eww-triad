use serde_json::{Value, json};

use crate::error::{Error, Result};

pub const TRIAD_IPC_VERSION: u64 = 1;

pub const NATIVE_QUERY_REQUESTS: &[&str] = &[
    "state",
    "capabilities",
    "workspaces",
    "outputs",
    "windows",
    "focused-window",
    "overview-state",
    "keyboard-layouts",
    "layout-state",
    "commands",
];

pub const NATIVE_EVENT_NAMES: &[&str] = &["state", "layout", "window"];

pub fn request(name: &str) -> Value {
    json!({"triad": {"version": TRIAD_IPC_VERSION, "request": name}})
}

pub fn query_request(name: &str) -> Result<Value> {
    if NATIVE_QUERY_REQUESTS.contains(&name) {
        Ok(request(name))
    } else {
        Err(Error::UnsupportedRequest(name.to_string()))
    }
}

pub fn event_stream_request() -> Value {
    event_stream_request_for(NATIVE_EVENT_NAMES).expect("default native events are valid")
}

pub fn event_stream_request_for(events: &[&str]) -> Result<Value> {
    validate_event_names(events)?;
    Ok(json!({
        "triad": {
            "version": TRIAD_IPC_VERSION,
            "request": "event-stream",
            "events": events
        }
    }))
}

pub fn validate_event_names(events: &[&str]) -> Result<()> {
    if events.is_empty() {
        return Err(Error::UnsupportedEvent("<empty>".to_string()));
    }
    for event in events {
        if !NATIVE_EVENT_NAMES.contains(event) {
            return Err(Error::UnsupportedEvent((*event).to_string()));
        }
    }
    Ok(())
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

pub fn dispatch_binding_request(kind: &str, binding: &str, value: Option<i64>) -> Result<Value> {
    if binding.is_empty() {
        return Err(Error::InvalidDispatchBinding(
            "binding must not be empty".to_string(),
        ));
    }

    let mut payload = serde_json::Map::new();
    payload.insert("version".to_string(), json!(TRIAD_IPC_VERSION));
    payload.insert("request".to_string(), json!("dispatch-binding"));
    payload.insert("kind".to_string(), json!(kind));
    payload.insert("binding".to_string(), json!(binding));

    match kind {
        "key" | "pointer" => {
            if value.is_some() {
                return Err(Error::InvalidDispatchBinding(format!(
                    "{kind} dispatch does not take a value"
                )));
            }
        }
        "axis" => {
            payload.insert("ticks".to_string(), json!(value.unwrap_or(1)));
        }
        "gesture" => {
            let Some(fingers) = value else {
                return Err(Error::InvalidDispatchBinding(
                    "gesture dispatch requires fingers".to_string(),
                ));
            };
            if fingers < 0 || fingers > u32::MAX as i64 {
                return Err(Error::InvalidDispatchBinding(
                    "gesture fingers must fit uint32".to_string(),
                ));
            }
            payload.insert("fingers".to_string(), json!(fingers));
        }
        other => {
            return Err(Error::InvalidDispatchBinding(format!(
                "unsupported binding kind: {other}"
            )));
        }
    }

    Ok(json!({"triad": Value::Object(payload)}))
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
    expect_reply_type(triad, "state")?;
    triad
        .get("state")
        .cloned()
        .ok_or_else(|| Error::Triad("state reply did not include state".to_string()))
}

pub fn expect_reply_type(reply: &Value, expected: &str) -> Result<()> {
    let actual = reply_type(reply).unwrap_or("<missing>");
    if actual == expected {
        Ok(())
    } else {
        Err(Error::UnexpectedReply {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

pub fn validate_query_reply(request_name: &str, value: &Value) -> Result<()> {
    let triad = reply_ok(value)?;
    expect_reply_type(triad, request_name)
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
    fn query_request_accepts_native_reads() {
        let value = query_request("capabilities").unwrap();
        assert_eq!(value["triad"]["request"], json!("capabilities"));
    }

    #[test]
    fn query_request_rejects_unknown_reads() {
        let err = query_request("not-real").unwrap_err();
        assert!(err.to_string().contains("not-real"));
    }

    #[test]
    fn event_stream_request_can_select_events() {
        let value = event_stream_request_for(&["state", "window"]).unwrap();
        assert_eq!(value["triad"]["events"], json!(["state", "window"]));
    }

    #[test]
    fn event_stream_request_rejects_unknown_events() {
        let err = event_stream_request_for(&["state", "bad"]).unwrap_err();
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn set_layout_can_target_workspace() {
        let value = set_layout_request("scroller", None, Some(3));
        assert_eq!(value["triad"]["request"], json!("set-layout"));
        assert_eq!(value["triad"]["target"]["workspace_idx"], json!(3));
    }

    #[test]
    fn dispatch_binding_key_omits_extra_fields() {
        let value = dispatch_binding_request("key", "Super+Return", None).unwrap();
        assert_eq!(value["triad"]["request"], json!("dispatch-binding"));
        assert_eq!(value["triad"]["kind"], json!("key"));
        assert_eq!(value["triad"]["binding"], json!("Super+Return"));
        assert!(value["triad"].get("ticks").is_none());
        assert!(value["triad"].get("fingers").is_none());
    }

    #[test]
    fn dispatch_binding_pointer_omits_extra_fields() {
        let value = dispatch_binding_request("pointer", "BTN_LEFT", None).unwrap();
        assert_eq!(value["triad"]["kind"], json!("pointer"));
        assert!(value["triad"].get("ticks").is_none());
        assert!(value["triad"].get("fingers").is_none());
    }

    #[test]
    fn dispatch_binding_axis_includes_ticks() {
        let value = dispatch_binding_request("axis", "WheelDown", Some(-2)).unwrap();
        assert_eq!(value["triad"]["ticks"], json!(-2));
    }

    #[test]
    fn dispatch_binding_axis_defaults_ticks() {
        let value = dispatch_binding_request("axis", "WheelDown", None).unwrap();
        assert_eq!(value["triad"]["ticks"], json!(1));
    }

    #[test]
    fn dispatch_binding_gesture_includes_fingers() {
        let value = dispatch_binding_request("gesture", "swipe-up", Some(3)).unwrap();
        assert_eq!(value["triad"]["fingers"], json!(3));
    }

    #[test]
    fn dispatch_binding_rejects_bad_payloads() {
        assert!(dispatch_binding_request("bad", "x", None).is_err());
        assert!(dispatch_binding_request("gesture", "swipe-up", None).is_err());
        assert!(dispatch_binding_request("key", "Super+Return", Some(1)).is_err());
    }

    #[test]
    fn state_reply_rejects_error_envelope() {
        let err = state_from_reply(&json!({"ok": false, "error": "bad"})).unwrap_err();
        assert!(err.to_string().contains("bad"));
    }

    #[test]
    fn representative_query_replies_match_expected_types() {
        for (name, reply) in [
            (
                "state",
                json!({"ok": true, "triad": {"version": 1, "type": "state", "state": {"version": 1, "layout": {}, "windows": []}}}),
            ),
            (
                "capabilities",
                json!({"ok": true, "triad": {"version": 1, "type": "capabilities", "capabilities": {"event_stream": true}}}),
            ),
            (
                "workspaces",
                json!({"ok": true, "triad": {"version": 1, "type": "workspaces", "workspaces": [{"tag_id": 1, "workspace_idx": 1}]}}),
            ),
            (
                "outputs",
                json!({"ok": true, "triad": {"version": 1, "type": "outputs", "outputs": [{"name": "DP-1"}]}}),
            ),
            (
                "windows",
                json!({"ok": true, "triad": {"version": 1, "type": "windows", "windows": [{"id": 7}]}}),
            ),
            (
                "focused-window",
                json!({"ok": true, "triad": {"version": 1, "type": "focused-window", "window": {"id": 7}}}),
            ),
            (
                "overview-state",
                json!({"ok": true, "triad": {"version": 1, "type": "overview-state", "overview": {"is_open": false}}}),
            ),
            (
                "keyboard-layouts",
                json!({"ok": true, "triad": {"version": 1, "type": "keyboard-layouts", "keyboard_layouts": {"names": ["us"], "current_idx": 0}}}),
            ),
            (
                "layout-state",
                json!({"ok": true, "triad": {"version": 1, "type": "layout-state", "state": {"active_tag": 1, "workspaces": []}}}),
            ),
            (
                "commands",
                json!({"ok": true, "triad": {"version": 1, "type": "commands", "catalog": {"version": 1, "commands": []}}}),
            ),
        ] {
            validate_query_reply(name, &reply).unwrap();
        }
    }
}
