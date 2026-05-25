use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Eww,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EwwState {
    pub schema: String,
    pub connected: bool,
    pub triad_state_version: Option<u64>,
    pub active_tag: Option<u64>,
    pub active_workspace_idx: Option<u64>,
    pub focused_window_id: Option<u64>,
    pub capabilities: Value,
    pub workspaces: Value,
    pub windows: Value,
    pub outputs: Value,
    pub layouts: Value,
    pub layout_cycle: Value,
    pub layout_cycle_entries: Value,
    pub overview: Value,
    pub keyboard_layouts: Value,
    pub current_keyboard_layout_idx: Option<u64>,
}

pub fn eww_state(state: &Value) -> EwwState {
    let layout = state.get("layout").unwrap_or(&Value::Null);
    let windows = state.get("windows").unwrap_or(&Value::Null);
    EwwState {
        schema: "eww-triad.v1".to_string(),
        connected: true,
        triad_state_version: state.get("version").and_then(Value::as_u64),
        active_tag: layout.get("active_tag").and_then(Value::as_u64),
        active_workspace_idx: layout.get("active_workspace_idx").and_then(Value::as_u64),
        focused_window_id: focused_window_id(windows),
        capabilities: state.get("capabilities").cloned().unwrap_or(Value::Null),
        workspaces: layout.get("workspaces").cloned().unwrap_or(Value::Null),
        windows: windows.clone(),
        outputs: state.get("outputs").cloned().unwrap_or(Value::Null),
        layouts: layout.get("layouts").cloned().unwrap_or(Value::Null),
        layout_cycle: layout.get("layout_cycle").cloned().unwrap_or(Value::Null),
        layout_cycle_entries: layout
            .get("layout_cycle_entries")
            .cloned()
            .unwrap_or(Value::Null),
        overview: state.get("overview").cloned().unwrap_or(Value::Null),
        keyboard_layouts: state
            .get("keyboard_layouts")
            .cloned()
            .unwrap_or(Value::Null),
        current_keyboard_layout_idx: state
            .get("current_keyboard_layout_idx")
            .and_then(Value::as_u64),
    }
}

pub fn disconnected_eww_state() -> EwwState {
    EwwState {
        schema: "eww-triad.v1".to_string(),
        connected: false,
        triad_state_version: None,
        active_tag: None,
        active_workspace_idx: None,
        focused_window_id: None,
        capabilities: json!({}),
        workspaces: json!([]),
        windows: json!([]),
        outputs: json!([]),
        layouts: json!([]),
        layout_cycle: json!([]),
        layout_cycle_entries: json!([]),
        overview: json!({}),
        keyboard_layouts: json!([]),
        current_keyboard_layout_idx: None,
    }
}

pub fn disconnected_state() -> Value {
    serde_json::to_value(disconnected_eww_state()).expect("EwwState serializes")
}

fn focused_window_id(windows: &Value) -> Option<u64> {
    windows.as_array()?.iter().find_map(|window| {
        if window.get("is_focused").and_then(Value::as_bool) == Some(true) {
            window.get("id").and_then(Value::as_u64)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
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
}
