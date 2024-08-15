use wasm_bindgen::prelude::*;
use liveview_native_core::{
    diff::fragment::{
        FragmentDiff,
        FragmentMerge,
        Root,
        RootDiff,
    },
};

#[wasm_bindgen(
inline_js = "
    export function map_to_object(map) {
        const out = Object.create(null);
        map.forEach((value, key) => {
            if (value instanceof Map) {
                out[key] = map_to_object(value)
            } else {
                out[key] = value
            }
        });
        return out;
    }"
    )
]
extern "C" {
    fn map_to_object(map: JsValue) -> JsValue;
}

#[wasm_bindgen]
pub struct Rendered {
    inner: Root,
}

#[derive(serde::Deserialize)]
pub struct RenderedExtractedInput {
    #[serde(rename = "r")]
    reply: Option<bool>,
    #[serde(rename = "t")]
    title: Option<String>,
    #[serde(rename = "e", default = "Vec::new")]
    events: Vec<String>,
    #[serde(flatten)]
    diff: RootDiff,
}
#[derive(serde::Serialize)]
pub struct RenderedExtractedOutput {
    reply: Option<bool>,
    title: Option<String>,
    events: Vec<String>,
    diff: RootDiff,
}
impl From<RenderedExtractedInput> for RenderedExtractedOutput {
    fn from(value: RenderedExtractedInput) -> Self {
        Self {
            reply: value.reply,
            title: value.title,
            events: value.events,
            diff: value.diff,
        }
    }
}

#[wasm_bindgen]
impl Rendered {
    #[wasm_bindgen(constructor)]
    pub fn new(view_id: i32, rendered: JsValue) -> Self {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Debug);
        let root_diff: RootDiff = serde_wasm_bindgen::from_value(rendered).unwrap();
        let root : Root = root_diff.try_into().expect("Failed to convert RootDiff into Root");
        Self {
            inner:root,
        }
    }
    pub fn mergeDiff(&mut self, diff: JsValue) {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff).unwrap();
        log::info!("DIFF: {diff:#?}");
        self.inner = self.inner.clone().merge(diff).expect("Failed to merge diff");
        log::info!("MERGED: {:#?}", self.inner);
    }
    pub fn isComponentOnlyDiff(&self, diff: JsValue) -> bool {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff).unwrap();
        let root : Root = diff.try_into().expect("Failed to convert RootDiff into Root");

        root.is_component_only_diff()
    }
    pub fn componentCIDs(&self, diff: JsValue) -> Vec<u32> {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff).unwrap();
        let root : Root = diff.try_into().expect("Failed to convert RootDiff into Root");
        root.component_cids()
    }
    pub fn getComponent(&self, diff: JsValue, cid: i32) -> JsValue {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff).unwrap();
        let root : Root = diff.try_into().expect("Failed to convert RootDiff into Root");
        let component = if let Some(component) = root.get_component(cid) {
            component
        } else {
            return JsValue::null();
        };
        serde_wasm_bindgen::to_value(&component).unwrap_or(JsValue::null())

    }
    pub fn isNewFingerprint(&self, diff: JsValue) -> bool {
        let diff: RootDiff = if let Ok(diff) = serde_wasm_bindgen::from_value(diff) {
            diff
        } else {
            return false;
        };
        let root : Root = if let Ok(root) = diff.try_into() {
            root
        } else {
            return false;
        };
        root.is_new_fingerprint()
    }
    pub fn get(&self) -> JsValue {
        let map = serde_wasm_bindgen::to_value(&self.inner).unwrap();
        map_to_object(map)
    }
    pub fn toString(&self) -> JsValue {
        let out = js_sys::Array::new();
        let rendered : String = self.inner.clone().try_into().expect("Failed to render root");
        out.push(&rendered.into());
        let streams  = js_sys::Set::default();
        out.push(&streams);

        out.into()
    }
    pub fn extract(diff: JsValue) -> JsValue {
        console_log::init_with_level(log::Level::Debug);
        let extracted : RenderedExtractedInput = serde_wasm_bindgen::from_value(diff).expect("Failed to extract diff");
        let extracted : RenderedExtractedOutput = extracted.into();
        let map = serde_wasm_bindgen::to_value(&extracted).unwrap();
        //map_to_object(map)
        map
    }
}

#[wasm_bindgen]
pub struct LiveSocket {
    //inner: liveview_native_core::live_socket::LiveSocket,
}
#[wasm_bindgen]
impl LiveSocket {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String, socket: JsValue) -> Self {
        Self {
        }
    }
    pub fn channel(&mut self, topic: String, params: JsValue) {
    }
}
#[wasm_bindgen]
pub struct Socket {
    inner: phoenix_channels_client::Socket,
}
