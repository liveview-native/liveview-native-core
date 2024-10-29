use std::collections::HashMap;

use phoenix_channels_client::url::UrlQuery;
use uniffi::Object;

use crate::dom::NodeRef;

use super::{channel::LiveFile, error::LiveSocketError, LiveChannel};

#[derive(uniffi::Object)]
pub enum FormValue {
    File(LiveFile),
    Value(String),
}

/// Contains abstractions for form data as well as traits for instrumenting form changes
/// and submissions
#[derive(uniffi::Object)]
pub struct FormModel {
    /// The dom element that this form corresponds to
    node_ref: NodeRef,
    /// The event name that triggers on `change` events
    on_change: Option<String>,
    /// The event name that triggers on `submit` events
    on_submit: Option<String>,
    /// The name of the form
    target_name: String,
    /// Current stringified name, value pairs for the inputs
    fields: HashMap<String, FormValue>,
}

impl FormModel {
    pub fn new(node_ref: NodeRef, target_name: String, fields: HashMap<String, FormValue>) -> Self {
        FormModel {
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

        // if needed send the message
        Ok(())
    }
}
