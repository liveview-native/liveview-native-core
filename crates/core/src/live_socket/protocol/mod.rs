pub mod event;

use super::{LiveChannel, LiveSocketError};
use crate::dom::NodeRef;
use event::{PhxEvent, UserEvent};
use phoenix_channels_client::{Event, Payload, JSON};

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveChannel {
    pub async fn send_event(
        &self,
        event: PhxEvent,
        value: Option<String>,
        sender: &NodeRef,
    ) -> Result<Payload, LiveSocketError> {
        let val = value.map(JSON::deserialize).transpose()?;
        self.send_event_json(event, val, sender).await
    }

    pub async fn send_event_json(
        &self,
        event: PhxEvent,
        value: Option<JSON>,
        sender: &NodeRef,
    ) -> Result<Payload, LiveSocketError> {
        let r#type = event.type_name().into();
        let change_attrs = event.loading_attr();

        let event = self
            .document
            .inner()
            .lock()
            .expect("lock poison")
            .get_attribute_by_name(*sender, event.phx_attribute().as_str())
            .and_then(|attr| attr.value)
            .ok_or(LiveSocketError::MissingEventAttribtue(
                event.type_name().to_string(),
            ))?;

        let default = serde_json::Value::Object(serde_json::Map::new());
        let payload = UserEvent {
            r#type,
            event,
            value: value.map(serde_json::Value::from).unwrap_or(default),
        };

        let val = serde_json::to_value(payload)?;

        self.lock_node(*sender, change_attrs);

        let user_event = Event::User {
            user: "event".into(),
        };

        let payload = Payload::JSONPayload {
            json: JSON::from(val),
        };

        let res = self.channel.call(user_event, payload, self.timeout).await;

        self.unlock_node(*sender, change_attrs);

        Ok(res?)
    }
}
