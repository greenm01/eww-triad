use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Eww,
    Raw,
}

#[derive(Debug, Serialize)]
pub struct EwwState<'a> {
    pub schema: &'static str,
    pub connected: bool,
    pub active_tag: Option<u64>,
    pub active_workspace_idx: Option<u64>,
    pub focused_window_id: Option<u64>,
    pub workspaces: &'a Value,
    pub windows: &'a Value,
    pub outputs: &'a Value,
    pub layouts: &'a Value,
    pub layout_cycle: &'a Value,
    pub overview: &'a Value,
    pub keyboard_layouts: &'a Value,
    pub current_keyboard_layout_idx: Option<u64>,
}

pub fn eww_state(state: &Value) -> EwwState<'_> {
    let layout = state.get("layout").unwrap_or(&Value::Null);
    let windows = state.get("windows").unwrap_or(&Value::Null);
    EwwState {
        schema: "eww-triad.v1",
        connected: true,
        active_tag: layout.get("active_tag").and_then(Value::as_u64),
        active_workspace_idx: layout.get("active_workspace_idx").and_then(Value::as_u64),
        focused_window_id: focused_window_id(windows),
        workspaces: layout.get("workspaces").unwrap_or(&Value::Null),
        windows,
        outputs: state.get("outputs").unwrap_or(&Value::Null),
        layouts: layout.get("layouts").unwrap_or(&Value::Null),
        layout_cycle: layout.get("layout_cycle").unwrap_or(&Value::Null),
        overview: state.get("overview").unwrap_or(&Value::Null),
        keyboard_layouts: state.get("keyboard_layouts").unwrap_or(&Value::Null),
        current_keyboard_layout_idx: state
            .get("current_keyboard_layout_idx")
            .and_then(Value::as_u64),
    }
}

pub fn disconnected_state() -> Value {
    json!({"schema": "eww-triad.v1", "connected": false})
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
            "layout": {
                "active_tag": 4,
                "active_workspace_idx": 2,
                "workspaces": [{"workspace_idx": 2}],
                "layouts": [{"id": "scroller"}],
                "layout_cycle": ["scroller"]
            },
            "windows": [{"id": 99, "is_focused": true}],
            "outputs": [{"name": "DP-1"}],
            "overview": {"is_open": false},
            "keyboard_layouts": ["us"],
            "current_keyboard_layout_idx": 0
        });
        let projected = eww_state(&state);
        assert_eq!(projected.schema, "eww-triad.v1");
        assert_eq!(projected.active_tag, Some(4));
        assert_eq!(projected.focused_window_id, Some(99));
    }
}
