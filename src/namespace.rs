use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// represents a single namespace level for traversion JSON structures.
///
/// # Example
/// `test.value` would be represented by two Namespace Object's `test` and `value`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Namespace {
    Object { id: String },
    Array { id: String, index: usize }, // TODO: look into making Array id an Option
}

impl Namespace {
    #![allow(dead_code)]
    pub(crate) fn as_object(&self) -> Option<&String> {
        match self {
            Namespace::Object { id } => Some(id),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<(&String, &usize)> {
        match self {
            Namespace::Array { id, index } => Some((id, index)),
            _ => None,
        }
    }

    pub(crate) fn is_object(&self) -> bool {
        match self {
            Namespace::Object { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn is_array(&self) -> bool {
        match self {
            Namespace::Array { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn id(&self) -> &String {
        match self {
            Namespace::Object { id } => &id,
            Namespace::Array { id, .. } => &id,
        }
    }

    /// parse takes an ordinary namespaced string eg. `object.nested[0][1].nested.field` and
    /// turns it into a usable namespace object for use in transformations.
    ///
    /// **NOTE:** This parser assumes `[` or `]` in the namespace denotes an array, if this is not true
    ///       you will have to manually create your own namespace; the backend transformer handles
    ///       the distinction, just the parser has no way of knowing the difference.
    ///
    pub fn parse<'a, S>(input: S) -> Result<Vec<Namespace>>
    where
        S: Into<Cow<'a, str>>,
    {
        input
            .into()
            .split(".")
            .flat_map(|s| s.split_terminator("]"))
            .map(|v| {
                if let Some(idx) = v.find("[") {
                    Ok(Namespace::Array {
                        id: v[..idx].to_string(),
                        index: v[idx + 1..].parse()?,
                    })
                } else {
                    Ok(Namespace::Object { id: v.to_string() })
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace() {
        let ns = "embedded.array[0][1]";
        let results = Namespace::parse(ns).unwrap();
        let expected = vec![
            Namespace::Object {
                id: String::from("embedded"),
            },
            Namespace::Array {
                id: String::from("array"),
                index: 0,
            },
            Namespace::Array {
                id: String::from(""),
                index: 1,
            },
        ];
        assert_eq!(expected, results);
    }

    #[test]
    fn test_blank() {
        let ns = "field";
        let results = Namespace::parse(ns).unwrap();
        let expected = vec![Namespace::Object {
            id: String::from("field"),
        }];
        assert_eq!(expected, results);

        let ns = "array-field[0]";
        let results = Namespace::parse(ns).unwrap();
        let expected = vec![Namespace::Array {
            id: String::from("array-field"),
            index: 0,
        }];
        assert_eq!(expected, results);
    }
}
