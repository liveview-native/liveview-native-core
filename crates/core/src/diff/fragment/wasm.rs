use super::*;

// These are used in the wasm build.
impl Root {
    pub fn set_new_render(&mut self, new: bool) {
        self.new_render = Some(new);
    }
    pub fn is_component_only_diff(&self) -> bool {
        !self.components.is_empty() && self.fragment.is_empty()
    }
    pub fn is_new_fingerprint(&self) -> bool {
        self.fragment.is_new_fingerprint()
    }
    pub fn get_component(&self, cid: i32) -> Option<Component> {
        self.components.get(&format!("{cid}")).cloned()
    }
    pub fn component_cids(&self) -> Vec<u32> {
        let keys: Vec<u32> = self
            .components
            .keys()
            .filter_map(|key| key.parse::<u32>().ok())
            .collect();

        keys
    }
}

impl Fragment {
    pub fn is_new_fingerprint(&self) -> bool {
        match self {
            Fragment::Regular { statics, .. } | Fragment::Comprehension { statics, .. } => {
                statics.is_some()
            }
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Fragment::Comprehension {
                dynamics,
                statics: None,
                reply: None,
                templates: None,
                stream: None,
            } => dynamics.is_empty(),
            _ => false,
        }
    }
}
