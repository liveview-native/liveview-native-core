use liveview_native_core::diff::fragment::{FragmentMerge, Root, RootDiff};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Rendered {
    inner: Root,
    view_id: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct RenderedExtractedInput {
    #[serde(rename = "r")]
    reply: Option<bool>,
    #[serde(rename = "t")]
    title: Option<String>,
    #[serde(rename = "e", default = "Vec::new")]
    events: Vec<String>,
    #[serde(rename = "fingerprint")]
    _finger_print: Option<i32>,
    #[serde(flatten)]
    diff: RootDiff,
}
#[derive(serde::Serialize, Debug)]
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
use log::{debug, info};

#[wasm_bindgen]
impl Rendered {
    #[wasm_bindgen(constructor)]
    pub fn new(view_id: String, rendered: JsValue) -> Result<Rendered, JsError> {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Debug);
        debug!("New rendered: {rendered:?}");
        let root_diff: RootDiff = serde_wasm_bindgen::from_value(rendered)?;
        let root: Root = root_diff.try_into()?;
        Ok(Rendered {
            inner: root,
            view_id,
        })
    }
    #[wasm_bindgen(js_name = "mergeDiff")]
    pub fn merge_diff(&mut self, diff: JsValue) -> Result<(), JsError> {
        debug!("Merging diff: {diff:?}");
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff)?;
        info!("DIFF: {diff:#?}");
        info!("BEFORE MERGE: {:#?}", self.inner);
        self.inner = self.inner.clone().merge(diff)?;
        info!("MERGED: {:#?}", self.inner);
        Ok(())
    }
    #[wasm_bindgen(js_name = "isComponentOnlyDiff")]
    pub fn is_component_only_diff(&self, diff: JsValue) -> Result<bool, JsError> {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff)?;
        let root: Root = diff.try_into()?;

        Ok(root.is_component_only_diff())
    }
    #[wasm_bindgen(js_name = "componentCIDs")]
    pub fn component_cids(&self, diff: JsValue) -> Result<Vec<u32>, JsError> {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff)?;
        let root: Root = diff.try_into()?;
        Ok(root.component_cids())
    }

    #[wasm_bindgen(js_name = "resetRender")]
    pub fn reset_render(&self, _cid: i32) -> Result<JsValue, JsError> {
        todo!()
    }

    #[wasm_bindgen(js_name = "getComponent")]
    pub fn get_component(&self, diff: JsValue, cid: i32) -> Result<JsValue, JsError> {
        let diff: RootDiff = serde_wasm_bindgen::from_value(diff)?;
        let root: Root = diff.try_into()?;
        let component = if let Some(component) = root.get_component(cid) {
            component
        } else {
            return Ok(JsValue::null());
        };
        Ok(serde_wasm_bindgen::to_value(&component)?)
    }
    #[wasm_bindgen(js_name = "isNewFingerprint")]
    pub fn is_new_fingerprint(&self, diff: JsValue) -> bool {
        let diff: RootDiff = if let Ok(diff) = serde_wasm_bindgen::from_value(diff) {
            diff
        } else {
            return false;
        };
        let root: Root = if let Ok(root) = diff.try_into() {
            root
        } else {
            return false;
        };
        root.is_new_fingerprint()
    }
    pub fn get(&self) -> Result<JsValue, JsError> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let map = self
            .inner
            .serialize(&serializer)
            .expect("Failed to serialize");
        Ok(map)
    }
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> Result<JsValue, JsError> {
        let out = js_sys::Array::new();
        let _ = console_log::init_with_level(log::Level::Debug);
        let rendered: String = self.inner.clone().try_into()?;
        out.push(&rendered.into());
        let streams = js_sys::Set::default();
        out.push(&streams);

        Ok(out.into())
    }
    #[wasm_bindgen(js_name = "parentViewId")]
    pub fn parent_view_id(&self) -> String {
        self.view_id.clone()
    }

    pub fn extract(diff: JsValue) -> Result<JsValue, JsError> {
        let extracted: RenderedExtractedInput = serde_wasm_bindgen::from_value(diff)?;
        let extracted: RenderedExtractedOutput = extracted.into();
        // This is needed because various fields in RootDiff won't be included.
        // The json compatible serializer is a bit more costly.
        // https://github.com/RReverser/serde-wasm-bindgen?tab=readme-ov-file#supported-types
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        let out = extracted
            .serialize(&serializer)
            .expect("Failed to serialize");
        Ok(out)
    }
}
