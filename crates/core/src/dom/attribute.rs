use std::fmt::{self, Write};

use smallstr::SmallString;

use crate::InternedString;

/// Represents the fully-qualified name of an attribute
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeName {
    /// This is used by svg attributes, e.g. `xlink-href`
    pub namespace: Option<InternedString>,
    pub name: InternedString,
}
impl fmt::Display for AttributeName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref ns) = self.namespace {
            write!(f, "{}:{}", ns, &self.name)
        } else {
            write!(f, "{}", &self.name)
        }
    }
}
impl AttributeName {
    #[inline]
    pub fn new<N: Into<InternedString>>(name: N) -> Self {
        Self {
            namespace: None,
            name: name.into(),
        }
    }

    #[inline]
    pub fn new_with_namespace<NS: Into<InternedString>, N: Into<InternedString>>(
        namespace: NS,
        name: N,
    ) -> Self {
        Self {
            namespace: Some(namespace.into()),
            name: name.into(),
        }
    }
}
impl From<&str> for AttributeName {
    fn from(s: &str) -> Self {
        match s.split_once(':') {
            None => AttributeName::new(s),
            Some((ns, name)) => AttributeName::new_with_namespace(ns, name),
        }
    }
}
impl From<InternedString> for AttributeName {
    fn from(s: InternedString) -> Self {
        AttributeName::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: AttributeName,
    pub value: AttributeValue,
}
impl Attribute {
    /// Creates a new attribute with the given name and value
    ///
    /// If the attribute you wish to create is namespaced, make sure to set the
    /// namespace with `set_namespace` after creating the attribute.
    pub fn new<K: Into<InternedString>>(name: K, value: AttributeValue) -> Self {
        Self {
            name: AttributeName::new(name),
            value,
        }
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

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::String(s) => Some(s.as_str()),
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
