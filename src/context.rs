//! The [`Context`] struct.

use crate::{core::Event, request::Object, response::Response};
use log::trace;
use rvk::{error::Error, methods::messages, objects::Integer, APIClient, Params};
use std::sync::{Arc, Mutex};

/// Stores information necessary for handlers, allows to send the resulting
/// message.
#[derive(Debug)]
pub struct Context {
    event: Event,
    object: Object,
    api: Arc<Mutex<APIClient>>,
    peer_id: Integer,
    response: Response,
}

impl Context {
    /// Creates a new [`Context`].
    ///
    /// # Panics
    /// - no user_id on object (only [`Event::MessageAllow`])
    /// - no message on object (all except [`Event::MessageAllow`])
    /// - no peer_id on object (all except [`Event::MessageAllow`] and [`Event::MessageTypingState`])
    pub fn new(event: Event, object: Object, api: Arc<Mutex<APIClient>>) -> Self {
        let peer_id = match event {
            Event::MessageAllow => object
                .user_id()
                .expect("no user_id on message_allow object"),
            Event::MessageTypingState => {
                object
                    .message()
                    .clone()
                    .expect("no message on message_typing_state object")
                    .from_id
            }
            _ => {
                object
                    .message()
                    .clone()
                    .expect("no message on object")
                    .peer_id
                    .expect("no peer_id on object")
            }
        };

        Self {
            event,
            object,
            api,
            peer_id,
            response: Response::new(),
        }
    }

    /// Returns the original Callback API event type that caused this handler to
    /// run.
    pub fn event(&self) -> Event {
        self.event
    }

    /// Returns the object associated with the event (given by Callback API).
    pub fn object(&self) -> &Object {
        &self.object
    }

    /// Returns an [`rvk::APIClient`], wrapped into
    /// [`Arc`][`std::sync::Arc`]`<`[`Mutex`][`std::sync::Mutex`]`<...>>`.
    pub fn api(&self) -> Arc<Mutex<APIClient>> {
        Arc::clone(&self.api)
    }

    /// Returns the current pending response object (mutable).
    pub fn response(&mut self) -> &mut Response {
        &mut self.response
    }

    /// Sends the response.
    ///
    /// This does not erase the response object. You can send multiple messages.
    ///
    /// This method currently blocks until the [`rvk::APIClient`] is available,
    /// so only one message is being sent at a given time. This behavior may
    /// change.
    pub fn send(&self) -> Result<(), Error> {
        let api = self.api.lock().map_err(|e| Error::Other(e.to_string()))?;
        let mut params = Params::new();

        params.insert("peer_id".into(), format!("{}", self.peer_id));

        let res = &self.response;
        let msg = res.message();
        let attachments = res.attachments();
        let kbd = res.keyboard();

        if !msg.is_empty() {
            params.insert("message".into(), msg.clone());
        }

        if !attachments.is_empty() {
            params.insert(
                "attachment".into(),
                attachments
                    .iter()
                    .map(|info| info.to_string())
                    .fold(String::new(), |acc, v| acc + "," + &v),
            );
        }

        if let Some(kbd) = kbd {
            params.insert(
                "keyboard".into(),
                serde_json::to_string(kbd).expect("failed to serialize keyboard"),
            );
        }

        let random_id: i32 = rand::random();
        params.insert("random_id".into(), format!("{}", random_id));

        trace!("sending message {:#?}", params);

        messages::send(&*api, params).map(|_| ())
    }
}
