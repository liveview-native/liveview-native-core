use std::{borrow::Cow, collections::HashMap};

use phoenix_channels_client::{Event, Payload, JSON};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ServerEvent {
    pub diff: Option<serde_json::Value>,
    #[serde(flatten)]
    pub misc: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct UserEvent {
    pub r#type: String,
    pub event: String,
    pub value: serde_json::Value,
}

impl UserEvent {
    pub fn new(r#type: String, event: String, value: Option<JSON>) -> Self {
        Self {
            r#type,
            event,
            value: value
                .map(serde_json::Value::from)
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
        }
    }

    pub fn to_channel_message(self) -> (Event, Payload) {
        let val = serde_json::to_value(self).expect("Failed to serialize UserEvent");

        (
            Event::User {
                user: "event".into(),
            },
            Payload::JSONPayload {
                json: JSON::from(val),
            },
        )
    }
}

#[derive(uniffi::Enum, Clone, Debug)]
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
    pub fn type_name(&self) -> Cow<'_, str> {
        match self {
            PhxEvent::Other(o) => Cow::Borrowed(o.as_str()),
            PhxEvent::PhxValue(var_name) => ["value-", var_name.as_str()].concat().into(),
            PhxEvent::PhxClick => "click".into(),
            PhxEvent::PhxClickAway => "click-away".into(),
            PhxEvent::PhxChange => "change".into(),
            PhxEvent::PhxSubmit => "submit".into(),
            PhxEvent::PhxFeedbackFor => "feedback-for".into(),
            PhxEvent::PhxFeedbackGroup => "feedback-group".into(),
            PhxEvent::PhxDisableWith => "disable-with".into(),
            PhxEvent::PhxTriggerAction => "trigger-action".into(),
            PhxEvent::PhxAutoRecover => "auto-recover".into(),
            PhxEvent::PhxBlur => "blur".into(),
            PhxEvent::PhxFocus => "focus".into(),
            PhxEvent::PhxWindowBlur => "window-blur".into(),
            PhxEvent::PhxWindowFocus => "window-focus".into(),
            PhxEvent::PhxKeydown => "keydown".into(),
            PhxEvent::PhxKeyup => "keyup".into(),
            PhxEvent::PhxWindowKeydown => "window-keydown".into(),
            PhxEvent::PhxWindowKeyup => "window-keyup".into(),
            PhxEvent::PhxKey => "key".into(),
            PhxEvent::PhxViewportTop => "viewport-top".into(),
            PhxEvent::PhxViewportBottom => "viewport-bottom".into(),
            PhxEvent::PhxMounted => "mounted".into(),
            PhxEvent::PhxUpdate => "update".into(),
            PhxEvent::PhxRemove => "remove".into(),
            PhxEvent::PhxHook => "hook".into(),
            PhxEvent::PhxConnected => "connected".into(),
            PhxEvent::PhxDisconnected => "disconnected".into(),
            PhxEvent::PhxDebounce => "debounce".into(),
            PhxEvent::PhxThrottle => "throttle".into(),
            PhxEvent::PhxTrackStatic => "track-static".into(),
        }
    }

    pub fn phx_attribute(&self) -> String {
        format!("phx-{}", self.type_name())
    }

    pub fn loading_attr(&self) -> Option<&str> {
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
