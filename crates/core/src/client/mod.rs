mod config;

pub use self::config::Config;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail};
use parking_lot::Mutex;
use phoenix_channels_client as phoenix_channels;
use phoenix_channels_client::{Payload, PhoenixEvent, Value};
use url::Url;

use crate::dom::{Document, NodeRef, Selector};
use crate::symbols;

type HashMap<K, V> =
    std::collections::HashMap<K, V, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

const PHX_PARENT_ID: &'static str = "data-phx-parent-id";
const PHX_MAIN: &'static str = "data-phx-main";
const PHX_ROOT_ID: &'static str = "data-phx-root-id";
const PHX_SESSION: &'static str = "data-phx-session";
const PHX_STATIC: &'static str = "data-phx-static";
const PHX_COMPONENT: &'static str = "data-phx-component";
const PHX_LIVE_LINK: &'static str = "data-phx-link";
const PHX_LINK_STATE: &'static str = "data-phx-link-state";
const PHX_REF: &'static str = "data-phx-ref";
const PHX_REF_SRC: &'static str = "data-phx-ref-src";
const PHX_SKIP: &'static str = "data-phx-skip";
const PHX_PRUNE: &'static str = "data-phx-prune";
const TRACK_STATIC: &'static str = "track-static";

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("client is already connected to a server")]
    AlreadyConnected,
    #[error("initial page load failed")]
    RequestError(#[from] reqwest::Error),
    #[error("dom parsing failed")]
    ParseError(#[from] crate::parser::ParseError),
    #[error("unable to locate csrf-token meta value, unable to proced")]
    MissingCsrfToken,
    #[error("phoenix channels client encountered an error")]
    SocketError(#[from] phoenix_channels::ClientError),
    #[error("channel encountered an error")]
    ChannelError(#[from] phoenix_channels::ChannelError),
}

#[derive(Debug, Copy, Clone, Default)]
pub enum NavigationMode {
    /// Navigation is disabled; the live view will stay connected to its initial URL
    #[default]
    Disabled,
    /// Navigation is limited to live redirects with `replace: true`
    ReplaceOnly,
    /// Navigation is unrestricted, and both replace and push redirects are allowed
    Enabled,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClientStatus {
    New,
    Loaded,
    Connecting,
    Connected,
    Disconnected,
}
impl ClientStatus {
    #[inline]
    pub fn is_connected(self) -> bool {
        self == Self::Connected
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(usize);

#[derive(Clone)]
pub struct LiveViewClient(Arc<Mutex<Client>>);

struct Client {
    status: ClientStatus,
    config: Config,
    csrf_token: String,
    client: phoenix_channels::Client,
    reloader: phoenix_channels::Client,
    doc: Document,
    view_ids: HashMap<String, ViewId>,
    views: Vec<Arc<Mutex<View>>>,
}

impl LiveViewClient {
    pub fn new(config: Config) -> Result<Self, ClientError> {
        let client = phoenix_channels::Client::new(config.liveview_socket_config())?;
        let reloader = phoenix_channels::Client::new(config.livereload_socket_config())?;
        Ok(Self(Arc::new(Mutex::new(Client {
            status: ClientStatus::New,
            config,
            csrf_token: "".to_string(),
            client,
            reloader,
            doc: Document::empty(),
            view_ids: HashMap::default(),
            views: Vec::new(),
        }))))
    }

    /// Connect Lifecycle
    ///
    /// * Load page
    /// * Connect LV socket
    /// * Connect LV reload socket (optional)
    /// * Detect root live views
    /// * Connect live views to their channels
    pub async fn connect(&self) -> Result<(), ClientError> {
        use std::collections::hash_map::Entry;

        // We need a clone of this handle so that we can refer to it in callbacks
        let this = self.clone();

        let mut client = self.0.lock();

        if client.status.is_connected() {
            return Err(ClientError::AlreadyConnected);
        }

        // The client status is Loaded when reloading/reconnecting
        let is_reload = client.status == ClientStatus::Loaded;

        client.status = ClientStatus::Connecting;

        // Perform initial page load
        let http_client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .gzip(true)
            .redirect(reqwest::redirect::Policy::limited(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let response = http_client.get(client.config.url.clone()).send().await?;
        let text = response.text().await?;

        // Load document from page contents
        let doc = Document::parse(text.as_str())?;

        // Extract LV configuration vars, such as the CSRF token
        let csrf_token = select_csrf_token(&doc).ok_or_else(|| ClientError::MissingCsrfToken)?;
        let live_reload_enabled = is_reload || live_reload_enabled(&doc);
        let connect_params = connect_params(&doc);

        client
            .client
            .config_mut()
            .url
            .query_pairs_mut()
            .append_pair("_csrf_token", csrf_token.as_str())
            .extend_pairs(connect_params.iter());
        if !is_reload && live_reload_enabled {
            client
                .reloader
                .config_mut()
                .url
                .query_pairs_mut()
                .append_pair("_csrf_token", csrf_token.as_str());
        }

        client.csrf_token = csrf_token;
        client.config.live_reload = live_reload_enabled;

        // Establish socket connection
        client.client.connect().await?;
        client.status = ClientStatus::Connected;
        client.config.callbacks.connected();

        // Establish live reloader connection, if applicable
        if !is_reload && live_reload_enabled {
            client.reloader.connect().await?;
            let channel = client.reloader.join("phoenix:live_reload", None).await?;
            let lvclient = this.clone();
            channel
                .on("assets_change", move |_, _| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(lvclient.reload()).unwrap();
                })
                .await?;
        }

        // Parse root live views, and join them
        //
        // [data-phx-session]:not([data-phx-parent-id])
        let selector = Selector::And(
            Box::new(Selector::Attribute(symbols::DataPhxSession.into())),
            Box::new(Selector::Not(Box::new(Selector::Attribute(
                symbols::DataPhxParentId.into(),
            )))),
        );
        let mut main_view = None;
        for node in doc.select(selector) {
            match doc.get_attribute_by_name(node, "id") {
                Some(attr) if attr.value != "" => {
                    let id = attr.value.to_string();
                    let view_id = client.views.len();
                    let view_id = match client.view_ids.entry(id.clone()) {
                        Entry::Vacant(entry) => {
                            let id = ViewId(view_id);
                            entry.insert(id);
                            id
                        }
                        Entry::Occupied(_) => continue,
                    };
                    let is_main = client
                        .doc
                        .get_attribute_by_name(node, PHX_MAIN)
                        .map(|attr| attr.value == "true")
                        .unwrap_or(false);
                    let topic = format!("lv:{}", &id);
                    let channel = client
                        .client
                        .join(topic.as_str(), Some(Duration::from_secs(5)))
                        .await?;
                    let view_doc = doc.subtree(node);
                    let session = get_session(&view_doc, node);
                    let static_id = get_static(&view_doc, node);
                    let view = Arc::new(Mutex::new(View {
                        id: view_id,
                        id_string: id.clone(),
                        client: this.clone(),
                        channel: Some(channel.clone()),
                        doc: view_doc,
                        parent: None,
                        root: None,
                        is_dead: false,
                        is_main,
                        is_connected: true,
                        redirect: false,
                        href: client.config.url.clone(),
                        children: vec![],
                        session,
                        r#static: static_id,
                        rendered: Root::default(),
                    }));
                    if is_main {
                        assert_eq!(main_view.replace(id), None);
                    }
                    let view_handle = view.clone();
                    channel
                        .on("diff", move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_diff(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on("redirect", move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_redirect(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on("live_patch", move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_live_patch(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on("live_redirect", move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_live_redirect(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on(PhoenixEvent::Join, move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_join(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on(PhoenixEvent::Error, move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_error(payload);
                        })
                        .await?;
                    let view_handle = view.clone();
                    channel
                        .on(PhoenixEvent::Close, move |_channel, payload| {
                            let mut guard = view_handle.lock();
                            guard.on_close(payload);
                        })
                        .await?;

                    client.views.push(view);
                }
                _ => continue,
            }
        }

        // Set up the root dead view, if needed
        if !doc.is_empty() {
            let selector = Selector::Tag(symbols::Body.into());
            let body = {
                let mut iter = doc.select(selector);
                iter.next()
            };
            if let Some(body) = body {
                let is_first_child_phx_view = client
                    .doc
                    .children(body)
                    .first()
                    .copied()
                    .map(|c| is_phx_view(&doc, c));
                if !is_phx_view(&doc, body) && !is_first_child_phx_view.unwrap_or(false) {
                    let id = client
                        .doc
                        .get_attribute_by_name(body, "id")
                        .and_then(|attr| {
                            if attr.value != "" {
                                Some(attr.value.to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| "".to_string());
                    let view_id = ViewId(client.views.len());
                    client.view_ids.insert(id.clone(), view_id);
                    let view_doc = doc.subtree(body);
                    let session = get_session(&view_doc, body);
                    let static_id = get_static(&view_doc, body);
                    let view = Arc::new(Mutex::new(View {
                        id: view_id,
                        id_string: id,
                        client: this,
                        channel: None,
                        doc: view_doc,
                        parent: None,
                        root: None,
                        is_dead: true,
                        is_main: main_view.is_none(),
                        is_connected: false,
                        redirect: false,
                        href: client.config.url.clone(),
                        children: vec![],
                        session,
                        r#static: static_id,
                        rendered: Root::default(),
                    }));
                    client.views.push(view);
                }
            }
        }

        client.doc = doc;

        client.config.callbacks.loaded();

        Ok(())
    }

    pub async fn reload(&self) -> Result<(), ClientError> {
        // Unload all of the views, disconnect all channels and the liveview socket
        self.disconnect().await?;
        // Reconnect
        self.connect().await
    }

    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let mut client = self.0.lock();
        client.status = ClientStatus::Loaded;
        for view in client.views.drain(..) {
            let mut view = view.lock();
            view.leave().await;
        }
        client.view_ids.clear();
        client.client.disconnect().await.ok();
        client.config.callbacks.disconnected();

        Ok(())
    }
}

pub struct View {
    /// Uniquely identifies a View within a LiveViewClient
    id: ViewId,
    /// The identifier associated with this view in the original Document
    id_string: String,
    /// A handle to the client which created/owns this view
    client: LiveViewClient,
    /// A handle to the channel for this view
    channel: Option<Arc<phoenix_channels::Channel>>,
    parent: Option<ViewId>,
    root: Option<ViewId>,
    doc: Document,
    href: Url,
    children: Vec<ViewId>,
    session: Option<String>,
    r#static: Option<String>,
    is_dead: bool,
    is_main: bool,
    is_connected: bool,
    redirect: bool,
    rendered: Root,
}
impl View {
    async fn leave(&mut self) {
        if let Some(channel) = self.channel.as_deref() {
            channel.leave().await;
        }
    }

    /// This function is invoked when joining the topic controlling this view
    fn on_join(&mut self, payload: &Payload) {
        // We always expect a JSON payload here, so panic otherwise
        let Payload::Value(ref value) = payload else { panic!("expected json value, got binary"); };

        // Parse the diff content
        let mut root: Root = value.try_into().unwrap();

        self.rendered = root;
    }

    /// This function is invoked when a 'diff' event comes across the channel
    fn on_diff(&mut self, payload: &Payload) {
        // We always expect a JSON payload here, so panic otherwise
        let Payload::Value(ref value) = payload else { panic!("expected json value, got binary"); };

        // Parse the diff content
        let mut diff: RootDiff = value.try_into().unwrap();

        // Dispatch events
        for event in diff.events.drain(..) {
            todo!("dispatch event: {:#?}", &event);
        }

        self.merge(diff)
    }

    /// This function is invoked when a 'redirect' event comes across the channel
    fn on_redirect(&mut self, _payload: &Payload) {
        todo!()
    }

    /// This function is invoked when a 'live_patch' event comes across the channel
    fn on_live_patch(&mut self, _payload: &Payload) {
        todo!()
    }

    /// This function is invoked when a 'live_redirect' event comes across the channel
    fn on_live_redirect(&mut self, _payload: &Payload) {
        todo!()
    }

    /// This function is invoked when a 'phx_error' event comes across the channel
    fn on_error(&mut self, _payload: &Payload) {
        todo!()
    }

    /// This function is invoked when a 'phx_close' event comes across the channel
    fn on_close(&mut self, _payload: &Payload) {
        todo!()
    }

    fn merge(&mut self, diff: RootDiff) {
        let mut doc = Document::empty();

        // Apply the changes
        self.rendered.merge(diff).unwrap();

        // Render the changes to a new document
        //let doc = self.rendered.to_document();

        // Merge the document into the current document
        todo!();
    }
}

const DYNAMICS: &'static str = "d";
const STATIC: &'static str = "s";
const COMPONENTS: &'static str = "c";
const EVENTS: &'static str = "e";
const REPLY: &'static str = "r";
const TITLE: &'static str = "t";
const TEMPLATES: &'static str = "p";
const STREAM: &'static str = "stream";

struct Root {
    fragment: Fragment,
    components: HashMap<u64, Component>,
    title: Option<String>,
    stream_id: Option<String>,
}
impl Root {
    fn merge(&mut self, mut diff: RootDiff) -> anyhow::Result<()> {
        use std::collections::hash_map::Entry;

        self.fragment.merge(diff.fragment)?;

        for (cid, cdiff) in diff.components.drain() {
            match self.components.entry(cid) {
                Entry::Vacant(entry) => {
                    entry.insert(cdiff.try_into()?);
                }
                Entry::Occupied(mut entry) => {
                    entry.get_mut().merge(cdiff)?;
                }
            }
        }

        if diff.title.is_some() {
            self.title = diff.title;
        }

        if diff.stream_id.is_some() {
            self.stream_id = diff.stream_id;
        }

        Ok(())
    }
}
impl Default for Root {
    fn default() -> Self {
        Self {
            fragment: Default::default(),
            components: HashMap::default(),
            title: None,
            stream_id: None,
        }
    }
}
impl TryFrom<&Value> for Root {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("expected fragment object, got: {:#?}", value); };

        let fragment = obj.try_into()?;

        let mut components = HashMap::<u64, Component>::default();
        if let Some(components_raw) = obj.get(COMPONENTS) {
            match components_raw {
                Value::Object(ref components_raw) => {
                    for (cid, component_raw) in components_raw.iter() {
                        let cid = cid.parse::<u64>().map_err(|_| {
                            anyhow!("invalid component id, unable to parse as integer: {}", cid)
                        })?;
                        let component = component_raw.try_into()?;
                        components.insert(cid, component);
                    }
                }
                other => bail!(
                    "invalid components diff, expected object, got: {:#?}",
                    other
                ),
            }
        }

        let title = obj
            .get(TITLE)
            .map(|v| match v {
                Value::String(title) => Ok(Some(title.clone())),
                Value::Null => Ok(None),
                other => Err(anyhow!("invalid title, expected string, got: {:#?}", other)),
            })
            .unwrap_or(Ok(None))?;

        Ok(Self {
            fragment,
            components,
            title,
            stream_id: None,
        })
    }
}

enum Fragment {
    Static(StaticFragment),
    Dynamic(DynamicFragment),
}
impl Fragment {
    fn merge(&mut self, diff: FragmentDiff) -> anyhow::Result<()> {
        match (self, diff) {
            (this, FragmentDiff::Replace(fragment)) => {
                *this = fragment;

                Ok(())
            }
            (Self::Static(old_frag), FragmentDiff::Static(new_frag)) => old_frag.merge(new_frag),
            (Self::Dynamic(old_frag), FragmentDiff::Dynamic(new_frag)) => old_frag.merge(new_frag),
            (_old, _new) => Err(anyhow!("diff/merge error, fragment type mismatch")),
        }
    }
}
impl Default for Fragment {
    fn default() -> Self {
        Self::Static(StaticFragment {
            statics: Static::Parts(vec![]),
            children: vec![],
        })
    }
}
impl TryFrom<&Value> for Fragment {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("expected fragment object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for Fragment {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        if obj.contains_key(DYNAMICS) {
            obj.try_into().map(Self::Dynamic)
        } else {
            obj.try_into().map(Self::Static)
        }
    }
}

struct StaticFragment {
    statics: Static,
    children: Vec<Child>,
}
impl StaticFragment {
    fn merge(&mut self, mut diff: HashMap<u64, ChildDiff>) -> anyhow::Result<()> {
        for (cid, cdiff) in diff.drain() {
            self.children[cid as usize].merge(cdiff)?;
        }

        Ok(())
    }
}
impl TryFrom<&Value> for StaticFragment {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("expected fragment object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for StaticFragment {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        let statics: Static = match obj.get(STATIC) {
            Some(s) => s.try_into()?,
            None => bail!("expected static, but none was present"),
        };

        let mut children = Vec::new();
        for (key, value) in obj.iter() {
            if let Some(child_id) = key.as_str().parse::<u64>().ok() {
                // The children should be in order because the underlying map is a BTree
                assert_eq!(child_id as usize, children.len());
                let child: Child = value.try_into()?;
                children.push(child);
            }
        }

        Ok(Self { statics, children })
    }
}

struct DynamicFragment {
    dynamics: Vec<Vec<Child>>,
    statics: Static,
    templates: HashMap<u64, Vec<String>>,
}
impl DynamicFragment {
    fn merge(&mut self, mut diff: DynamicDiff) -> anyhow::Result<()> {
        use std::collections::hash_map::Entry;

        self.dynamics = diff
            .dynamics
            .drain(..)
            .map(|mut children| children.drain(..).map(TryInto::try_into).try_collect())
            .try_collect()?;
        for (tid, mut tmpls) in diff.templates.drain() {
            match self.templates.entry(tid) {
                Entry::Vacant(entry) => {
                    entry.insert(tmpls);
                }
                Entry::Occupied(mut entry) => {
                    let current_tmpls = entry.get_mut();
                    current_tmpls.clear();
                    current_tmpls.append(&mut tmpls);
                }
            }
        }

        Ok(())
    }
}
impl TryFrom<&Value> for DynamicFragment {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("expected fragment object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for DynamicFragment {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        static DEFAULT_STATIC: Value = Value::Array(vec![]);

        if !obj.contains_key(DYNAMICS) {
            bail!("invalid dynamic fragment, no dynamic children found");
        }

        let mut dynamics = Vec::new();
        match obj.get(DYNAMICS).unwrap() {
            Value::Array(ref dynamics_raw) => {
                for dynamic_raw in dynamics_raw.iter() {
                    match dynamic_raw {
                        Value::Array(ref dynamic_raw) => {
                            let mut dynamic: Vec<Child> = vec![];
                            for d in dynamic_raw.iter() {
                                dynamic.push(d.try_into()?);
                            }
                            dynamics.push(dynamic);
                        }
                        other => bail!("invalid dynamic child, expected array, got: {:#?}", other),
                    }
                }
            }
            other => {
                bail!(
                    "invalid dynamic fragment, expected array, got: {:#?}",
                    other
                );
            }
        }

        let statics: Static = obj.get(STATIC).unwrap_or(&DEFAULT_STATIC).try_into()?;

        let mut templates = HashMap::default();
        if let Some(tmpls) = obj.get(TEMPLATES) {
            match tmpls {
                Value::Null => (),
                Value::Object(ref tmpls) => {
                    for (tid, tmpl) in tmpls.iter() {
                        let tid = tid.parse::<u64>().map_err(|_| {
                            anyhow!("invalid template id, unable to parse as integer: {}", tid)
                        })?;
                        match tmpl {
                            Value::Array(ref tmpls) => {
                                let tmpls = tmpls
                                    .iter()
                                    .map(|v| {
                                        v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                                            anyhow!(
                                                "invalid template, expected string, got: {:#?}",
                                                v
                                            )
                                        })
                                    })
                                    .try_collect::<Vec<String>>()?;
                                templates.insert(tid, tmpls);
                            }
                            _ => bail!(
                                "invalid templates, expected array of strings, got: {:#?}",
                                tmpl
                            ),
                        }
                    }
                }
                other => bail!("expected template array, got: {:#?}", other),
            }
        }

        Ok(Self {
            dynamics,
            statics,
            templates,
        })
    }
}

enum Static {
    Parts(Vec<String>),
    Template(u64),
}
impl TryFrom<&Value> for Static {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) if n.is_u64() => Ok(Self::Template(n.as_u64().unwrap())),
            Value::String(ref id) => {
                match id.parse::<u64>().ok() {
                    Some(id) => Ok(Self::Template(id)),
                    None => Err(anyhow!("invalid template id, expected integer, or string parseable as integer, got: {:#?}", id)),
                }
            }
            Value::Array(ref raw_parts) => {
                let mut parts = Vec::with_capacity(raw_parts.len());
                for part in raw_parts {
                    match part {
                        Value::String(ref s) => parts.push(s.clone()),
                        _ => return Err(anyhow!("invalid static part, expected string, got: {:#?}", part)),
                    }
                }
                Ok(Self::Parts(parts))
            }
            _ => Err(anyhow!("invalid static, expected template id or array of strings, got: {:#?}", value)),
        }
    }
}

enum ComponentStatic {
    Parts(Vec<String>),
    Component(i64),
}
impl TryFrom<&Value> for ComponentStatic {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) if n.is_i64() => Ok(Self::Component(n.as_i64().unwrap())),
            Value::String(ref id) => {
                match id.parse::<i64>().ok() {
                    Some(id) => Ok(Self::Component(id)),
                    None => Err(anyhow!("invalid component id, expected integer, or string parseable as integer, got: {:#?}", id)),
                }
            }
            Value::Array(ref raw_parts) => {
                let mut parts = Vec::with_capacity(raw_parts.len());
                for part in raw_parts {
                    match part {
                        Value::String(ref s) => parts.push(s.clone()),
                        _ => return Err(anyhow!("invalid static part, expected string, got: {:#?}", part)),
                    }
                }
                Ok(Self::Parts(parts))
            }
            _ => Err(anyhow!("invalid static, expected component id or array of strings, got: {:#?}", value)),
        }
    }
}

enum Child {
    Fragment(Fragment),
    Component(u64),
    String(String),
}
impl Child {
    fn merge(&mut self, diff: ChildDiff) -> anyhow::Result<()> {
        match (self, diff) {
            (Self::Fragment(ref mut old_frag), ChildDiff::Fragment(new_frag)) => {
                old_frag.merge(new_frag);
            }
            (this, ChildDiff::Component(id)) => {
                *this = Self::Component(id);
            }
            (this, ChildDiff::String(s)) => {
                *this = Self::String(s);
            }
            (this, ChildDiff::Fragment(frag)) => {
                match frag {
                    FragmentDiff::Replace(f) => {
                        *this = Self::Fragment(f);
                    }
                    _ => bail!("invalid diff/merge, operation would require creating child, an invalid operation"),
                }
            }
        }

        Ok(())
    }
}
impl TryFrom<&Value> for Child {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) if n.is_u64() => Ok(Self::Component(n.as_u64().unwrap())),
            Value::String(ref s) => Ok(Self::String(s.clone())),
            other => other.try_into().map(Self::Fragment),
        }
    }
}

struct Component {
    statics: ComponentStatic,
    children: Vec<Child>,
}
impl Component {
    fn merge(&mut self, diff: ComponentDiff) -> anyhow::Result<()> {
        match diff {
            ComponentDiff::Replace(c) => {
                // In diffs, the backend sends negative cids as x-refs for component statics to indicate that it
                // refers to a component that already exists on the client (and positives are x-refs to component statics
                // that are only now being sent in the diff).
                //
                // The JS client cares about this distinction because it replaces static refs with the actual array of strings
                // from the referenced component.
                //
                // We do not; we always look up the refrenced component when building strings, so we change the cid to be non-negative.
                self.statics = match c.statics {
                    ComponentStatic::Component(cid) if cid < 0 => {
                        ComponentStatic::Component(cid.abs())
                    }
                    other => other,
                };
                self.children = c.children;
            }
            ComponentDiff::Update(mut components) => {
                for (cid, cdiff) in components.drain() {
                    self.children[cid as usize].merge(cdiff)?;
                }
            }
        }

        Ok(())
    }
}
impl TryFrom<&Value> for Component {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("expected component object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for Component {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        let statics: ComponentStatic = match obj.get(STATIC) {
            Some(s) => s.try_into()?,
            None => bail!("expected static, but none was present"),
        };

        let mut children = Vec::new();
        for (key, value) in obj.iter() {
            if let Some(child_id) = key.as_str().parse::<u64>().ok() {
                // The children should be in order because the underlying map is a BTree
                assert_eq!(child_id as usize, children.len());
                let child: Child = value.try_into()?;
                children.push(child);
            }
        }

        Ok(Self { statics, children })
    }
}

#[derive(Debug)]
struct Event {
    event: String,
    payload: Value,
}
impl TryFrom<&Value> for Event {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(ref values) => match values.as_slice() {
                [Value::String(ref event_name), payload] => Ok(Self {
                    event: event_name.clone(),
                    payload: payload.clone(),
                }),
                _ => bail!(
                    "invalid event, expected array of [event_name, payload], got: {:#?}",
                    value
                ),
            },
            _ => bail!(
                "invalid event, expected array of [event_name, payload], got: {:#?}",
                value
            ),
        }
    }
}

struct RootDiff {
    fragment: FragmentDiff,
    components: HashMap<u64, ComponentDiff>,
    events: Vec<Event>,
    reply: Option<Value>,
    title: Option<String>,
    stream_id: Option<String>,
}
impl TryFrom<&Value> for RootDiff {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("invalid root fragment diff, expected object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for RootDiff {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        let fragment: FragmentDiff = obj.try_into()?;

        let reply = obj.get(REPLY).cloned();

        let title = obj
            .get(TITLE)
            .map(|v| match v {
                Value::String(title) => Ok(Some(title.clone())),
                Value::Null => Ok(None),
                other => Err(anyhow!("invalid title, expected string, got: {:#?}", other)),
            })
            .unwrap_or(Ok(None))?;

        let events = obj
            .get(EVENTS)
            .map(|v| match v {
                Value::Array(events) => events
                    .iter()
                    .map(|e| e.try_into())
                    .try_collect::<Vec<Event>>(),
                Value::Null => Ok(vec![]),
                other => Err(anyhow!("invalid events, expected array, got: {:#?}", other)),
            })
            .unwrap_or_else(|| Ok(vec![]))?;

        if let Some(components_raw) = obj.get(COMPONENTS) {
            match components_raw {
                Value::Object(ref components_raw) => {
                    let mut components = HashMap::default();

                    for (cid, component_raw) in components_raw.iter() {
                        let cid = cid.parse::<u64>().map_err(|_| {
                            anyhow!(
                                "invalid component id, unable to parse integer from: {}",
                                cid
                            )
                        })?;
                        let cdiff: ComponentDiff = component_raw.try_into()?;
                        components.insert(cid, cdiff);
                    }

                    Ok(Self {
                        fragment,
                        components,
                        reply,
                        title,
                        events,
                        stream_id: None,
                    })
                }
                Value::Null => Ok(Self {
                    fragment,
                    components: HashMap::default(),
                    reply,
                    title,
                    events,
                    stream_id: None,
                }),
                other => bail!(
                    "invalid components diff, expected object, got: {:#?}",
                    other
                ),
            }
        } else {
            Ok(Self {
                fragment,
                components: HashMap::default(),
                reply,
                title,
                events,
                stream_id: None,
            })
        }
    }
}

enum FragmentDiff {
    Replace(Fragment),
    Static(HashMap<u64, ChildDiff>),
    Dynamic(DynamicDiff),
}
impl TryFrom<&Value> for FragmentDiff {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("invalid fragment diff, expected object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for FragmentDiff {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        if obj.contains_key(STATIC) {
            obj.try_into().map(Self::Replace)
        } else if obj.contains_key(DYNAMICS) {
            obj.try_into().map(Self::Dynamic)
        } else {
            let mut children = HashMap::default();

            for (key, value) in obj.iter() {
                if let Some(child_id) = key.as_str().parse::<u64>().ok() {
                    let child: ChildDiff = value.try_into()?;
                    children.insert(child_id, child);
                }
            }

            Ok(Self::Static(children))
        }
    }
}

struct DynamicDiff {
    dynamics: Vec<Vec<ChildDiff>>,
    templates: HashMap<u64, Vec<String>>,
}
impl TryFrom<&Value> for DynamicDiff {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("invalid dynamic diff, expected object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for DynamicDiff {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        let mut templates = HashMap::default();
        match obj.get(TEMPLATES) {
            None | Some(Value::Null) => (),
            Some(Value::Object(ref tmpls_obj)) => {
                for (tid, tmpls) in tmpls_obj.iter() {
                    let tid = tid.parse::<u64>().map_err(|_| {
                        anyhow!("invalid template id, unable to parse as integer: {}", &tid)
                    })?;
                    match tmpls {
                        Value::Array(ref tmpls) => {
                            let tmpls = tmpls
                                .iter()
                                .map(|v| {
                                    v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                                        anyhow!("invalid template, expected string, got: {:#?}", &v)
                                    })
                                })
                                .try_collect()?;
                            templates.insert(tid, tmpls);
                        }
                        other => bail!("invalid template, expected string, got: {:#?}", other),
                    }
                }
            }
            other => bail!(
                "invalid dynamic diff, expected templates array, got: {:#?}",
                other
            ),
        }

        let mut dynamics = Vec::new();
        match obj.get(DYNAMICS).unwrap() {
            Value::Array(ref dynamics_raw) => {
                for dynamic_raw in dynamics_raw.iter() {
                    match dynamic_raw {
                        Value::Array(ref dynamic_raw) => {
                            let mut dynamic: Vec<ChildDiff> = vec![];
                            for d in dynamic_raw.iter() {
                                dynamic.push(d.try_into()?);
                            }
                            dynamics.push(dynamic);
                        }
                        other => bail!(
                            "invalid dynamic child diffs, expected array, got: {:#?}",
                            other
                        ),
                    }
                }
            }
            other => {
                bail!(
                    "invalid dynamic fragment, expected array, got: {:#?}",
                    other
                );
            }
        }

        Ok(Self {
            dynamics,
            templates,
        })
    }
}

enum ComponentDiff {
    Replace(Component),
    Update(HashMap<u64, ChildDiff>),
}
impl TryInto<Component> for ComponentDiff {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Component, Self::Error> {
        match self {
            Self::Replace(c) => Ok(c),
            _ => Err(anyhow!("cannot create child from update fragment")),
        }
    }
}
impl TryFrom<&Value> for ComponentDiff {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let Value::Object(ref obj) = value else { bail!("invalid component diff, expected object, got: {:#?}", value); };

        obj.try_into()
    }
}
impl TryFrom<&serde_json::Map<String, Value>> for ComponentDiff {
    type Error = anyhow::Error;

    fn try_from(obj: &serde_json::Map<String, Value>) -> Result<Self, Self::Error> {
        assert!(!obj.contains_key(DYNAMICS));

        if obj.contains_key(STATIC) {
            obj.try_into().map(Self::Replace)
        } else {
            let mut children = HashMap::default();

            for (key, value) in obj.iter() {
                if let Some(child_id) = key.as_str().parse::<u64>().ok() {
                    let child: ChildDiff = value.try_into()?;
                    children.insert(child_id, child);
                }
            }

            Ok(Self::Update(children))
        }
    }
}

enum ChildDiff {
    Fragment(FragmentDiff),
    Component(u64),
    String(String),
}
impl TryInto<Child> for ChildDiff {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Child, Self::Error> {
        match self {
            Self::String(s) => Ok(Child::String(s)),
            Self::Component(cid) => Ok(Child::Component(cid)),
            Self::Fragment(FragmentDiff::Replace(f)) => Ok(Child::Fragment(f)),
            _ => Err(anyhow!("cannot create child from update fragment")),
        }
    }
}
impl TryFrom<&Value> for ChildDiff {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Number(n) if n.is_u64() => Ok(Self::Component(n.as_u64().unwrap())),
            Value::String(ref s) => Ok(Self::String(s.clone())),
            _ => value.try_into().map(Self::Fragment),
        }
    }
}

#[inline]
fn is_phx_view(doc: &Document, node: NodeRef) -> bool {
    doc.get_attribute_by_name(node, PHX_SESSION).is_some()
}

/// Select the value of the `content` attribute in `<meta name="csrf-token" content=".." />`
fn select_csrf_token(doc: &Document) -> Option<String> {
    let selector = Selector::And(
        Box::new(Selector::Tag(symbols::Meta.into())),
        Box::new(Selector::AttributeValue(
            symbols::Name.into(),
            "csrf-token".into(),
        )),
    );
    let mut nodes = doc.select(selector);
    let node = nodes.next()?;
    let attr = doc.get_attribute_by_name(node, "content");
    attr.map(|attr| attr.value.to_string())
}

/// Determines if an iframe whose src is `/phoenix/live_reload/frame` is present
fn live_reload_enabled(doc: &Document) -> bool {
    let selector = Selector::And(
        Box::new(Selector::Tag(symbols::Iframe.into())),
        Box::new(Selector::AttributeValue(
            symbols::Src.into(),
            "/phoenix/live_reload/frame".into(),
        )),
    );
    let mut nodes = doc.select(selector);
    nodes.next().is_some()
}

fn get_session(doc: &Document, node: NodeRef) -> Option<String> {
    match doc.get_attribute_by_name(node, PHX_SESSION) {
        Some(attr) if attr.value != "" => Some(attr.value.to_string()),
        _ => None,
    }
}

fn get_static(doc: &Document, node: NodeRef) -> Option<String> {
    match doc.get_attribute_by_name(node, PHX_STATIC) {
        Some(attr) if attr.value != "" => Some(attr.value.to_string()),
        _ => None,
    }
}

fn connect_params(doc: &Document) -> HashMap<String, String> {
    let mut params = HashMap::default();

    let selector = Selector::Attribute(symbols::TrackStatic.into());
    let mut manifest = vec![];
    for node in doc.select(selector) {
        match doc.get_attribute_by_name(node, "src") {
            Some(attr) if attr.value != "" => manifest.push(attr.value.to_string()),
            _ => match doc.get_attribute_by_name(node, "href") {
                Some(attr) if attr.value != "" => manifest.push(attr.value.to_string()),
                _ => (),
            },
        }
    }
    if !manifest.is_empty() {
        params.insert(
            "_track_static".to_string(),
            serde_json::to_string(&manifest).unwrap(),
        );
    }
    params.insert("_mounts".to_string(), "0".to_string());
    params.insert("_live_referer".to_string(), "".to_string());

    params
}
