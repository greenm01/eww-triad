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
