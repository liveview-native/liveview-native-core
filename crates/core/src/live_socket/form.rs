use std::collections::HashMap;

use phoenix_channels_client::url::UrlQuery;
use uniffi::Object;

use crate::dom::NodeRef;

use super::{channel::LiveFile, error::LiveSocketError, LiveChannel};

pub enum FormValue {
    File(LiveFile),
    Value(String),
}

/// Contains abstractions for form data as well as traits for instrumenting form changes
/// and submissions
pub struct Form {
    /// The dom element that this form corresponds to
    node_ref: NodeRef,
    /// The event name that triggers on `phx-change` events
    on_change: Option<String>,
    /// The event name that triggers on `phx-submit` events
    on_submit: Option<String>,
    /// The name of the form
    target_name: String,
    /// Current stringified name, value pairs for the inputs
    fields: HashMap<String, FormValue>,
}

impl Form {
    pub fn new(node_ref: NodeRef, target_name: String, fields: HashMap<String, FormValue>) -> Self {
        Form {
            on_change: None,
            on_submit: None,
            node_ref,
            target_name,
            fields,
        }
    }

    /// Triggers a submit event with the current values
    pub async fn submit(&self, channel: &LiveChannel) -> Result<(), LiveSocketError> {
        // send the message, upload the chunks in a loop
        Ok(())
    }

    /// Updates the values in the form, if there is an `phx-change` event
    /// hook set up, it attempts to send one.
    pub async fn update_values<S, I>(
        &mut self,
        values: I,
        channel: &LiveChannel,
    ) -> Result<(), LiveSocketError>
    where
        S: AsRef<str>,
        I: Iterator<Item = (S, FormValue)>,
    {
        // apply the changes
        for (k, v) in values {
            self.fields.insert(k.as_ref().into(), v);
        }

        // if needed send the message
        if let Some(event_name) = self.on_change.as_ref() {}

        Ok(())
    }
}
