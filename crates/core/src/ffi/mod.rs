mod support;

pub use support::{AttributeVec, RustResult, RustSlice, RustStr, RustString};

use crate::{
    diff::PatchResult,
    dom::{self, NodeRef},
    diff::fragment::RootDiff,
};

#[repr(C)]
pub struct Node<'a> {
    pub ty: NodeType,
    pub data: NodeData<'a>,
}
impl<'a> Node<'a> {
    fn from(doc: &'a dom::Document, node: NodeRef) -> Self {
        match doc.get(node) {
            dom::Node::Root => Self {
                ty: NodeType::Root,
                data: NodeData { root: () },
            },
            dom::Node::Leaf(ref s) => Self {
                ty: NodeType::Leaf,
                data: NodeData {
                    leaf: RustStr::from_str(s.as_str()),
                },
            },
            dom::Node::Element(ref elem) => {
                let attrs = elem.attributes();
                let mut attributes = Vec::with_capacity(attrs.len());
                for attr in attrs {
                    attributes.push(Attribute::from(attr));
                }
                Self {
                    ty: NodeType::Element,
                    data: NodeData {
                        element: Element {
                            namespace: elem
                                .name
                                .namespace
                                .map(|ns| RustStr::from_str(ns.as_str()))
                                .unwrap_or_default(),
                            tag: RustStr::from_str(elem.name.name.as_str()),
                            attributes: AttributeVec::from_vec(attributes),
                        },
                    },
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NodeType {
    Root = 0,
    Element = 1,
    Leaf = 2,
}

#[repr(C)]
pub union NodeData<'a> {
    pub root: (),
    pub element: Element<'a>,
    pub leaf: RustStr<'a>,
}

#[repr(C)]
pub struct OptionNodeRef {
    pub is_some: bool,
    pub some_value: NodeRef,
}
impl OptionNodeRef {
    fn some(value: NodeRef) -> Self {
        Self {
            is_some: true,
            some_value: value,
        }
    }
    fn none() -> Self {
        Self {
            is_some: false,
            some_value: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Element<'a> {
    pub namespace: RustStr<'static>,
    pub tag: RustStr<'static>,
    pub attributes: AttributeVec<'a>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Attribute<'a> {
    pub namespace: RustStr<'static>,
    pub name: RustStr<'static>,
    pub value: RustStr<'a>,
}
impl<'a> Attribute<'a> {
    fn from(attr: &'a dom::Attribute) -> Self {
        Self {
            namespace: attr
                .name
                .namespace
                .map(|ns| RustStr::from_str(ns.as_str()))
                .unwrap_or_default(),
            name: RustStr::from_str(attr.name.name.as_str()),
            value: attr
                .value
                .as_str()
                .map(|v| RustStr::from_str(v))
                .unwrap_or_default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum ChangeType {
    Change = 0,
    Add = 1,
    Remove = 2,
    Replace = 3,
}

#[export_name = "__liveview_native_core$Document$drop"]
pub extern "C" fn document_drop(doc: *mut dom::Document) {
    unsafe {
        drop(Box::from_raw(doc));
    }
}

#[export_name = "__liveview_native_core$Document$empty"]
pub extern "C" fn document_empty() -> *mut dom::Document {
    let document = Box::new(dom::Document::empty());
    Box::into_raw(document)
}

#[export_name = "__liveview_native_core$Document$parse"]
pub extern "C" fn document_parse<'a>(
    text: RustStr<'a>,
    error: *mut RustString,
) -> support::RustResult {
    match dom::Document::parse(text.to_str()) {
        Ok(doc) => {
            let doc = Box::new(doc);
            support::RustResult {
                is_ok: true,
                ok_result: Box::into_raw(doc) as *mut std::ffi::c_void,
            }
        }
        Err(err) => {
            unsafe {
                error.write(RustString::from_string(err.to_string()));
            }
            support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            }
        }
    }
}

#[export_name = "__liveview_native_core$Document$to_string"]
pub extern "C" fn document_to_string(doc: *mut dom::Document) -> RustString {
    let doc = unsafe { &*doc };
    RustString::from_string(doc.to_string())
}

#[export_name = "__liveview_native_core$Document$node_to_string"]
pub extern "C" fn document_node_to_string(doc: *mut dom::Document, node: NodeRef) -> RustString {
    let doc = unsafe { &*doc };
    let mut buf = String::new();
    doc.print_node(node, &mut buf, dom::PrintOptions::Pretty)
        .expect("error printing node");
    RustString::from_string(buf)
}

#[export_name = "__liveview_native_core$Document$merge"]
pub extern "C" fn document_merge(
    doc: *mut dom::Document,
    other: *const dom::Document,
    handler: extern "C-unwind" fn(*mut (), ChangeType, NodeRef, OptionNodeRef) -> (),
    context: *mut (),
) {
    let doc = unsafe { &mut *doc };
    let other = unsafe { &*other };
    let patches = crate::diff::diff(doc, other);
    if patches.is_empty() {
        return;
    }

    let mut editor = doc.edit();
    let mut stack = vec![];
    for patch in patches.into_iter() {
        let patch_result = patch.apply(&mut editor, &mut stack);
        match patch_result {
            None => (),
            Some(PatchResult::Add { node, parent }) => {
                handler(context, ChangeType::Add, node, OptionNodeRef::some(parent));
            }
            Some(PatchResult::Remove { node, parent }) => {
                handler(
                    context,
                    ChangeType::Remove,
                    node,
                    OptionNodeRef::some(parent),
                );
            }
            Some(PatchResult::Change { node }) => {
                handler(context, ChangeType::Change, node, OptionNodeRef::none());
            }
            Some(PatchResult::Replace { node, parent }) => {
                handler(
                    context,
                    ChangeType::Replace,
                    node,
                    OptionNodeRef::some(parent),
                );
            }
        }
    }
    editor.finish();
}
#[export_name = "__liveview_native_core$Document$parse_fragment_json"]
pub extern "C" fn document_parse_fragment_json<'a>(
    text: RustStr<'a>,
    error: *mut RustString,
) -> support::RustResult {
    match dom::Document::parse_fragment_json(text.to_str().to_string()) {
        Ok(doc) => {
            let doc = Box::new(doc);
            support::RustResult {
                is_ok: true,
                ok_result: Box::into_raw(doc) as *mut std::ffi::c_void,
            }
        }
        Err(err) => {
            unsafe {
                error.write(RustString::from_string(err.to_string()));
            }
            support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            }
        }
    }
}
#[export_name = "__liveview_native_core$Document$merge_fragment_json"]
pub extern "C" fn document_merge_fragment_json<'a>(
    doc: *mut dom::Document,
    other_json: RustStr<'a>,
    handler: extern "C-unwind" fn(*mut (), ChangeType, NodeRef, OptionNodeRef) -> (),
    context: *mut (),
    error: *mut RustString,
) -> support::RustResult {
    let other_json = other_json.to_str();
    let root_diff : Result<RootDiff, _> = serde_json::from_str(other_json);
    let root_diff = match root_diff {
        Ok(fragment) => fragment,

        Err(err) => {
            unsafe {
                error.write(RustString::from_string(err.to_string()));
            }
            return support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            };
        }
    };
    let doc = unsafe { &mut *doc };
    if let Err(err) = doc.merge_fragment(root_diff) {
        unsafe {
            error.write(RustString::from_string(err.to_string()));
        }
        return support::RustResult {
            is_ok: false,
            ok_result: std::ptr::null_mut(),
        };
    }
    let new_root = if let Some(fragment) = doc.fragment_template.clone() {
        fragment
    } else {
        unsafe {
            error.write(RustString::from_string("Fragment template is None!".to_string()));
        }
        return support::RustResult {
            is_ok: false,
            ok_result: std::ptr::null_mut(),
        };
    };

    let other_doc : String = match new_root.try_into() {
        Ok(rendered) => rendered,
        Err(err) => {
            unsafe {
                error.write(RustString::from_string(err.to_string()));
            }
            return support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            };
        }
    };
    let other_doc = match dom::Document::parse(other_doc) {
        Ok(doc) => doc,
        Err(err) => {
            unsafe {
                error.write(RustString::from_string(err.to_string()));
            }
            return support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            };
        }
    };
    let other_doc = Box::new(other_doc);
    document_merge(doc, Box::into_raw(other_doc), handler, context);
    support::RustResult {
        is_ok: true,
        ok_result: std::ptr::null_mut(),
    }
}

#[export_name = "__liveview_native_core$Document$root"]
pub extern "C" fn document_root(doc: *const dom::Document) -> NodeRef {
    assert!(!doc.is_null());

    let doc = unsafe { &*doc };
    doc.root()
}

#[export_name = "__liveview_native_core$Document$get"]
pub extern "C" fn document_get_node<'a>(doc: *const dom::Document, node: NodeRef) -> Node<'a> {
    let doc = unsafe { &*doc };
    Node::from(doc, node)
}

#[export_name = "__liveview_native_core$Document$children"]
pub extern "C" fn document_get_children<'a>(
    doc: *const dom::Document,
    node: NodeRef,
) -> RustSlice<'a, NodeRef> {
    let doc = unsafe { &*doc };
    RustSlice::from_slice(doc.children(node))
}

#[export_name = "__liveview_native_core$Document$attributes"]
pub extern "C" fn document_get_attributes(
    doc: *const dom::Document,
    node: NodeRef,
) -> AttributeVec<'static> {
    let doc = unsafe { &*doc };
    let attrs = doc.attributes(node);
    let mut result = Vec::with_capacity(attrs.len());
    for attr in attrs {
        result.push(Attribute::from(attr));
    }
    AttributeVec::from_vec(result)
}

#[export_name = "__liveview_native_core$Document$get_parent"]
pub extern "C" fn document_get_parent(doc: *const dom::Document, node: NodeRef) -> OptionNodeRef {
    let doc = unsafe { &*doc };
    match doc.parent(node) {
        Some(parent) => OptionNodeRef::some(parent),
        None => OptionNodeRef::none(),
    }
}
