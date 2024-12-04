use std::borrow::Cow;

#[derive(uniffi::Enum, Clone)]
pub enum PhxEvent {
    Other(String),
    PhxValue(String),
    PhxClick,
    PhxClickAway,
    PhxChange,
    PhxSubmit,
    PhxFeedbackFor,
    PhxFeedbackGroup,
    PhxDisableWith,
    PhxTriggerAction,
    PhxAutoRecover,
    PhxBlur,
    PhxFocus,
    PhxWindowBlur,
    PhxWindowFocus,
    PhxKeydown,
    PhxKeyup,
    PhxWindowKeydown,
    PhxWindowKeyup,
    PhxKey,
    PhxViewportTop,
    PhxViewportBottom,
    PhxMounted,
    PhxUpdate,
    PhxRemove,
    PhxHook,
    PhxConnected,
    PhxDisconnected,
    PhxDebounce,
    PhxThrottle,
    PhxTrackStatic,
}

impl PhxEvent {
    fn str_name<'a>(&'a self) -> Cow<'a, str> {
        match self {
            PhxEvent::Other(o) => Cow::Borrowed(o.as_str()),
            PhxEvent::PhxValue(var_name) => ["phx-value-", var_name.as_str()].concat().into(),
            PhxEvent::PhxClick => "phx-click".into(),
            PhxEvent::PhxClickAway => "phx-click-away".into(),
            PhxEvent::PhxChange => "phx-change".into(),
            PhxEvent::PhxSubmit => "phx-submit".into(),
            PhxEvent::PhxFeedbackFor => "phx-feedback-for".into(),
            PhxEvent::PhxFeedbackGroup => "phx-feedback-group".into(),
            PhxEvent::PhxDisableWith => "phx-disable-with".into(),
            PhxEvent::PhxTriggerAction => "phx-trigger-action".into(),
            PhxEvent::PhxAutoRecover => "phx-auto-recover".into(),
            PhxEvent::PhxBlur => "phx-blur".into(),
            PhxEvent::PhxFocus => "phx-focus".into(),
            PhxEvent::PhxWindowBlur => "phx-window-blur".into(),
            PhxEvent::PhxWindowFocus => "phx-window-focus".into(),
            PhxEvent::PhxKeydown => "phx-keydown".into(),
            PhxEvent::PhxKeyup => "phx-keyup".into(),
            PhxEvent::PhxWindowKeydown => "phx-window-keydown".into(),
            PhxEvent::PhxWindowKeyup => "phx-window-keyup".into(),
            PhxEvent::PhxKey => "phx-key".into(),
            PhxEvent::PhxViewportTop => "phx-viewport-top".into(),
            PhxEvent::PhxViewportBottom => "phx-viewport-bottom".into(),
            PhxEvent::PhxMounted => "phx-mounted".into(),
            PhxEvent::PhxUpdate => "phx-update".into(),
            PhxEvent::PhxRemove => "phx-remove".into(),
            PhxEvent::PhxHook => "phx-hook".into(),
            PhxEvent::PhxConnected => "phx-connected".into(),
            PhxEvent::PhxDisconnected => "phx-disconnected".into(),
            PhxEvent::PhxDebounce => "phx-debounce".into(),
            PhxEvent::PhxThrottle => "phx-throttle".into(),
            PhxEvent::PhxTrackStatic => "phx-track-static".into(),
        }
    }

    fn loading_attr(&self) -> Option<&str> {
        match self {
            PhxEvent::PhxClick => Some("phx-click-loading"),
            PhxEvent::PhxChange => Some("phx-change-loading"),
            PhxEvent::PhxSubmit => Some("phx-submit-loading"),
            PhxEvent::PhxFocus => Some("phx-focus-loading"),
            PhxEvent::PhxBlur => Some("phx-blur-loading"),
            PhxEvent::PhxWindowKeydown => Some("phx-keydown-loading"),
            PhxEvent::PhxWindowKeyup => Some("phx-keyup-loading"),
            _ => None,
        }
    }
}
