use serde_json::Value;

use crate::protocol;

#[derive(Debug, Default, Clone)]
pub struct TriadState {
    state: Option<Value>,
}

impl TriadState {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn replace(&mut self, state: Value) {
        self.state = Some(state);
    }

    pub fn current(&self) -> Option<&Value> {
        self.state.as_ref()
    }

    pub fn apply_event(&mut self, event: &Value) -> bool {
        match protocol::event_name(event) {
            Some("state-changed") => self.apply_state_changed(event),
            Some("layout-state-changed") => self.apply_layout_changed(event),
            Some("window-changed") => self.apply_window_changed(event),
            _ => false,
        }
    }

    fn apply_state_changed(&mut self, event: &Value) -> bool {
        let Some(state) = protocol::event_payload(event).and_then(|payload| payload.get("state"))
        else {
            return false;
        };
        self.replace(state.clone());
        true
    }

    fn apply_layout_changed(&mut self, event: &Value) -> bool {
        let Some(layout) = protocol::event_payload(event).and_then(|payload| payload.get("state"))
        else {
            return false;
        };
        let Some(state) = self.state.as_mut().and_then(Value::as_object_mut) else {
            return false;
        };
        state.insert("layout".to_string(), layout.clone());
        true
    }

    fn apply_window_changed(&mut self, event: &Value) -> bool {
        let Some(window) = protocol::event_payload(event).and_then(|payload| payload.get("window"))
        else {
            return false;
        };
        let Some(id) = window.get("id").and_then(Value::as_u64) else {
            return false;
        };
        let Some(windows) = self
            .state
            .as_mut()
            .and_then(Value::as_object_mut)
            .and_then(|state| state.get_mut("windows"))
            .and_then(Value::as_array_mut)
        else {
            return false;
        };
        if let Some(existing) = windows
            .iter_mut()
            .find(|candidate| candidate.get("id").and_then(Value::as_u64) == Some(id))
        {
            *existing = window.clone();
        } else {
            windows.push(window.clone());
        }
        true
    }
}

#[cfg(test)]
mod tests;
