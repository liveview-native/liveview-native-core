use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}
use liveview_native_core::diff::fragment::{
    FragmentDiff,
    FragmentMerge,
    Root,
    RootDiff,
};

#[wasm_bindgen]
pub struct Rendered {
    inner: Root,
}

#[wasm_bindgen]
impl Rendered {
    #[wasm_bindgen(constructor)]
    pub fn new(view_id: i32, rendered: JsValue) -> Self {
		let root_diff: RootDiff = serde_wasm_bindgen::from_value(rendered).unwrap();
        let root : Root = root_diff.try_into().expect("Failed to convert RootDiff into Root");
        Self {
            inner:root,
        }
    }
    pub fn mergeDiff(&mut self, diff: JsValue) {
		let diff: RootDiff = serde_wasm_bindgen::from_value(diff).unwrap();
        self.inner = self.inner.clone().merge(diff).expect("Failed to merge diff");
    }
    pub fn isNewFingerprint(&self) -> bool {
        todo!();
    }
    pub fn get(&self) -> JsValue {
        todo!();

    }
    pub fn toString(&self) -> Vec<String> {
        let out : String = self.inner.clone().try_into().expect("Failed to render root");
        vec![out, String::new()]
    }
}
