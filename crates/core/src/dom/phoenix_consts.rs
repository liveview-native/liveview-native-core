use super::{Attribute, AttributeName};

#[macro_export]
macro_rules! const_attr {
    ($name:ident, $attr_name:expr, $value:expr) => {
        const $name: $crate::Attribute = $crate::Attribute {
            name: $crate::AttributeName {
                namespace: None,
                name: $attr_name.to_string(),
            },
            value: Some($value.to_string()),
        };
    };
    ($name:ident, $attr_name:expr) => {
        const $name: $crate::Attribute = $crate::Attribute {
            name: $crate::AttributeName {
                namespace: None,
                name: $attr_name.to_string(),
            },
            value: None,
        };
    };
}
