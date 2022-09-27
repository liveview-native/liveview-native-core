use std::fmt::{self, Write};

use cranelift_entity::{self as entity, entity_impl};
use smallstr::SmallString;

use crate::InternedString;

pub type AttributeList = entity::EntityList<AttributeRef>;
pub type AttributeListPool = entity::ListPool<AttributeRef>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct AttributeRef(u32);
entity_impl!(AttributeRef, "attr");
impl Default for AttributeRef {
    #[inline]
    fn default() -> Self {
        use cranelift_entity::packed_option::ReservedValue;

        Self::reserved_value()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// This is used by svg attributes, e.g. `xlink-href`
    pub namespace: Option<InternedString>,
    pub name: InternedString,
    pub value: AttributeValue,
}
impl Attribute {
    /// Creates a new attribute with the given name and value
    ///
    /// If the attribute you wish to create is namespaced, make sure to set the
    /// namespace with `set_namespace` after creating the attribute.
    pub fn new<K: Into<InternedString>>(name: K, value: AttributeValue) -> Self {
        Self {
            namespace: None,
            name: name.into(),
            value,
        }
    }

    #[inline]
    pub fn set_namespace<NS: Into<InternedString>>(&mut self, namespace: NS) {
        self.namespace.replace(namespace.into());
    }

    #[inline]
    pub fn set_value(&mut self, value: AttributeValue) {
        self.value = value;
    }
}

#[derive(Debug, Clone)]
pub enum AttributeValue {
    None,
    String(SmallString<[u8; 16]>),
}
impl AttributeValue {
    pub fn to_string(&self) -> String {
        match self {
            Self::None => String::new(),
            Self::String(s) => s.to_string(),
        }
    }
}
impl From<&str> for AttributeValue {
    #[inline]
    fn from(s: &str) -> Self {
        Self::String(SmallString::from_str(s))
    }
}
impl From<String> for AttributeValue {
    #[inline]
    fn from(s: String) -> Self {
        Self::String(SmallString::from_string(s))
    }
}
impl From<SmallString<[u8; 16]>> for AttributeValue {
    #[inline]
    fn from(s: SmallString<[u8; 16]>) -> Self {
        Self::String(s)
    }
}
impl Eq for AttributeValue {}
impl PartialEq for AttributeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::None, Self::String(s)) | (Self::String(s), Self::None) => s == "",
            (Self::String(x), Self::String(y)) => x == y,
        }
    }
}
impl PartialEq<str> for AttributeValue {
    fn eq(&self, other: &str) -> bool {
        match (self, other) {
            (Self::None, y) => y == "",
            (Self::String(x), y) => x == y,
        }
    }
}
impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::String(s) => {
                // Ensure we print strings quoted, with proper escaping of inner quotes
                f.write_char('"')?;
                let mut escaped = false;
                for c in s.chars() {
                    match c {
                        // Force unescaped quotes to be escaped
                        '"' if !escaped => write!(f, "{}", '"'.escape_default())?,
                        // Handle escape transitions
                        c if escaped => {
                            f.write_char('\\')?;
                            f.write_char(c)?;
                            escaped = false;
                        }
                        '\\' => {
                            escaped = true;
                            continue;
                        }
                        // Otherwise print characters normally
                        c => f.write_char(c)?,
                    }
                }
                f.write_char('"')
            }
        }
    }
}

pub struct AttributeIter<'a> {
    pub(super) attrs: &'a entity::PrimaryMap<AttributeRef, Attribute>,
    pub(super) refs: &'a [AttributeRef],
}
impl<'a> Iterator for AttributeIter<'a> {
    type Item = &'a Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        match self.refs.take_first().copied() {
            None => None,
            Some(attr) => Some(&self.attrs[attr]),
        }
    }
}
impl<'a> core::iter::FusedIterator for AttributeIter<'a> {}
impl<'a> core::iter::ExactSizeIterator for AttributeIter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.refs.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }
}
