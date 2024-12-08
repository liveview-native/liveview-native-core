pub mod event;

use super::{LiveChannel, LiveSocketError};
use crate::dom::NodeRef;
use event::{PhxEvent, ServerEvent, UserEvent};
use phoenix_channels_client::{Payload, JSON};
use serde::Deserialize;

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
        let event_attr = self
            .document
            .inner()
            .try_lock()?
            .get_attribute_by_name(*sender, event.phx_attribute().as_str())
            .and_then(|attr| attr.value)
            .ok_or(LiveSocketError::MissingEventAttribtue(
                event.type_name().to_string(),
            ))?;

        self.lock_node(*sender, event.loading_attr());

        let user_event = UserEvent::new(event.type_name().into(), event_attr, value);
        let (user_event, payload) = user_event.into_channel_message();
        let res = self.channel.call(user_event, payload, self.timeout).await;

        self.unlock_node(*sender, event.loading_attr());

        if let Ok(Payload::JSONPayload { json }) = &res {
            let val = serde_json::Value::from(json.clone());
            if let Ok(server_event) = ServerEvent::deserialize(val) {
                self.handle_server_event(server_event)?;
            } else {
                log::error!("Could not convert response into server event!")
            }
        }

        Ok(res?)
    }
}
