mod support;

use crate::dom::{self, AttributeRef, NodeRef};

pub use support::{RustResult, RustSlice, RustStr, RustString};

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
            dom::Node::Element(ref elem) => Self {
                ty: NodeType::Element,
                data: NodeData {
                    element: Element {
                        namespace: elem
                            .namespace
                            .map(|ns| RustStr::from_str(ns.as_str()))
                            .unwrap_or_default(),
                        tag: RustStr::from_str(elem.tag.as_str()),
                        attributes: RustSlice::from_slice(doc.attributes(node)),
                    },
                },
            },
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
#[derive(Copy, Clone)]
pub struct Element<'a> {
    pub namespace: RustStr<'static>,
    pub tag: RustStr<'static>,
    pub attributes: RustSlice<'a, AttributeRef>,
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
                .namespace
                .map(|ns| RustStr::from_str(ns.as_str()))
                .unwrap_or_default(),
            name: RustStr::from_str(attr.name.as_str()),
            value: attr
                .value
                .as_str()
                .map(|v| RustStr::from_str(v))
                .unwrap_or_default(),
        }
    }
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
    error: *mut support::RustString,
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
                error.write(support::RustString::from_string(err.to_string()));
            }
            support::RustResult {
                is_ok: false,
                ok_result: std::ptr::null_mut(),
            }
        }
    }
}

#[export_name = "__liveview_native_core$Document$merge"]
pub extern "C" fn document_merge(doc: *mut dom::Document, other: *const dom::Document) -> bool {
    let doc = unsafe { &mut *doc };
    let other = unsafe { &*other };
    let mut patches = crate::diff::diff(doc, other);
    if patches.is_empty() {
        return false;
    }

    let mut editor = doc.edit();
    let mut stack = vec![];
    for patch in patches.drain(..) {
        patch.apply(&mut editor, &mut stack);
    }
    editor.finish();
    true
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
pub extern "C" fn document_get_attributes<'a>(
    doc: *const dom::Document,
    node: NodeRef,
) -> RustSlice<'a, AttributeRef> {
    let doc = unsafe { &*doc };
    RustSlice::from_slice(doc.attributes(node))
}

#[export_name = "__liveview_native_core$Document$get_attribute"]
pub extern "C" fn document_get_attribute<'a>(
    doc: *const dom::Document,
    attr: AttributeRef,
) -> Attribute<'a> {
    let doc = unsafe { &*doc };
    Attribute::from(doc.get_attribute(attr))
}
