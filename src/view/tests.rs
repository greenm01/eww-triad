use serde_json::json;

use super::*;

#[test]
fn projection_lifts_common_bar_fields() {
    let state = json!({
        "version": 11,
        "capabilities": {"event_stream": true},
        "layout": {
            "active_tag": 4,
            "active_workspace_idx": 2,
            "workspaces": [{"workspace_idx": 2}],
            "layouts": [{"id": "scroller"}],
            "layout_cycle": ["scroller"],
            "layout_cycle_entries": [{"kind": "builtin", "id": "scroller"}]
        },
        "windows": [{"id": 99, "is_focused": true}],
        "outputs": [{"name": "DP-1"}],
        "overview": {"is_open": false},
        "keyboard_layouts": ["us"],
        "current_keyboard_layout_idx": 0
    });
    let projected = eww_state(&state);
    assert_eq!(projected.schema, "eww-triad.v1");
    assert_eq!(projected.triad_state_version, Some(11));
    assert_eq!(projected.active_tag, Some(4));
    assert_eq!(projected.focused_window_id, Some(99));
    assert_eq!(projected.capabilities["event_stream"], json!(true));
    assert_eq!(projected.layout_cycle_entries[0]["id"], json!("scroller"));
}

#[test]
fn disconnected_payload_keeps_public_shape() {
    let disconnected = disconnected_state();
    let connected = serde_json::to_value(eww_state(&json!({}))).unwrap();

    let disconnected_keys = disconnected.as_object().unwrap().keys().collect::<Vec<_>>();
    let connected_keys = connected.as_object().unwrap().keys().collect::<Vec<_>>();
    assert_eq!(disconnected_keys, connected_keys);
    assert_eq!(disconnected["connected"], json!(false));
    assert_eq!(disconnected["workspaces"], json!([]));
    assert_eq!(disconnected["overview"], json!({}));
}

#[test]
fn eww_state_round_trips_through_json() {
    let state = disconnected_eww_state();
    let encoded = serde_json::to_string(&state).unwrap();
    let decoded: EwwState = serde_json::from_str(&encoded).unwrap();
    assert!(!decoded.connected);
    assert_eq!(decoded.schema, "eww-triad.v1");
}
