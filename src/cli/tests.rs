use super::*;

#[test]
fn parse_events_accepts_csv() {
    assert_eq!(
        parse_events("state, window").unwrap(),
        vec!["state".to_string(), "window".to_string()]
    );
}

#[test]
fn parse_events_rejects_unknown_event() {
    let err = parse_events("state,bad").unwrap_err();
    assert!(err.to_string().contains("bad"));
}

#[test]
fn binding_kind_serializes_to_native_kind() {
    assert_eq!(BindingKind::Key.as_str(), "key");
    assert_eq!(BindingKind::Pointer.as_str(), "pointer");
    assert_eq!(BindingKind::Axis.as_str(), "axis");
    assert_eq!(BindingKind::Gesture.as_str(), "gesture");
}
