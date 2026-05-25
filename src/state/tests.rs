use serde_json::json;

use super::*;

#[test]
fn layout_event_updates_cached_layout() {
    let mut state = TriadState::new();
    state.replace(json!({"layout": {"active_tag": 1}, "windows": []}));
    let changed = state.apply_event(&json!({
        "triad": {"event": "layout-state-changed", "state": {"active_tag": 2}}
    }));
    assert!(changed);
    assert_eq!(state.current().unwrap()["layout"]["active_tag"], json!(2));
}

#[test]
fn window_event_replaces_matching_window() {
    let mut state = TriadState::new();
    state.replace(json!({"layout": {}, "windows": [{"id": 7, "title": "old"}]}));
    let changed = state.apply_event(&json!({
        "triad": {"event": "window-changed", "window": {"id": 7, "title": "new"}}
    }));
    assert!(changed);
    assert_eq!(
        state.current().unwrap()["windows"][0]["title"],
        json!("new")
    );
}
