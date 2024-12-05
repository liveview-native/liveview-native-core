pub mod event;

use super::{LiveChannel, LiveSocketError};
use crate::dom::NodeRef;
use event::PhxEvent;
use phoenix_channels_client::{Event, Payload, JSON};

#[cfg_attr(not(target_family = "wasm"), uniffi::export(async_runtime = "tokio"))]
impl LiveChannel {
    pub async fn send_event(
        &self,
        event: PhxEvent,
        payload: String,
        sender: &NodeRef,
    ) -> Result<Payload, LiveSocketError> {
        let val = JSON::deserialize(payload)?;
        self.send_event_json(event, val, sender).await
    }

    pub async fn send_event_json(
        &self,
        event: PhxEvent,
        payload: JSON,
        sender: &NodeRef,
    ) -> Result<Payload, LiveSocketError> {
        let change_attrs = event.loading_attr();

        self.lock_node(*sender, change_attrs);

        let user_event = Event::from(&event);
        let payload = Payload::JSONPayload { json: payload };

        let res = self.channel.call(user_event, payload, self.timeout).await;

        self.unlock_node(*sender, change_attrs);

        Ok(res?)
    }
}
