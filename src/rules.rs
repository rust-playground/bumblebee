use crate::errors::{Error, Result};
use crate::namespace::Namespace;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::fmt::Debug;

#[typetag::serde]
pub trait Rule: Debug {
    fn apply(&self, from: &Value, to: &mut Map<String, Value>) -> Result<()>;
}

///
/// Mapping is the type of transformation we will be attempting
///
#[derive(Debug, Serialize, Deserialize)]
pub enum Mapping<'a> {
    Direct {
        from: Cow<'a, str>,
        to: Cow<'a, str>,
    },
    Constant {
        from: Value,
        to: Cow<'a, str>,
    },
    Flatten {
        from: Cow<'a, str>,
        to: Cow<'a, str>,
        prefix: Option<Cow<'a, str>>,
        separator: Option<Cow<'a, str>>,
        recursive: bool,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Transform {
    source: Source,
    destination: Destination,
}

#[typetag::serde]
impl Rule for Transform {
    fn apply(&self, from: &Value, to: &mut Map<String, Value>) -> Result<()> {
        // when no value is found setting null
        // TODO: think about if we want to set null, returns error or just opt out.
        // thinking adding a required value to the from's would error and opt-out otherwise.

        let field = match &self.source {
            Source::Direct(id) => {
                if let Some(obj) = from.as_object() {
                    obj.get(id).unwrap_or(&Value::Null).clone()
                } else {
                    Value::Null
                }
            }
            Source::DirectArray { id, index } => match from {
                Value::Object(v) => {
                    if let Some(arr) = v.get(id) {
                        arr.get(index).unwrap_or(&Value::Null).clone()
                    } else {
                        Value::Null
                    }
                }
                Value::Array(v) => v.get(*index).unwrap_or(&Value::Null).clone(),
                _ => Value::Null,
            },
            Source::Constant(v) => v.clone(),
        };
        match &self.destination {
            Destination::Direct { id, namespace } => {
                get_last(namespace, to).insert(id.clone(), field);
            }
            Destination::DirectArray {
                id,
                namespace,
                index,
            } => {
                let current = get_last(namespace, to);
                if let Some(v) = current.get_mut(id) {
                    if let Some(arr) = v.as_array_mut() {
                        if *index >= arr.len() {
                            arr.resize_with(*index + 1, Value::default);
                        }
                        arr[*index] = field;
                    }
                } else {
                    // new array
                    let mut new_arr = vec![Value::Null; *index];
                    new_arr.push(field);
                    current.insert(id.clone(), Value::Array(new_arr));
                }
            }
            Destination::FlattenDirect {
                id,
                namespace,
                recursive,
                prefix,
                separator,
            } => {
                let p = match prefix {
                    Some(v) => v.clone(),
                    _ => String::from(""),
                };
                let s = match separator {
                    Some(v) => v.clone(),
                    _ => String::from(""),
                };
                match id {
                    Some(id) => {
                        let mut m = Map::new();
                        flatten(&s, &p, &field, &mut m, *recursive);
                        get_last(namespace, to).insert(id.clone(), Value::Object(m));
                    }
                    None => {
                        flatten(&s, &p, &field, get_last(namespace, to), *recursive);
                    }
                }
            }
            Destination::FlattenArray {
                id,
                namespace,
                prefix,
                index,
                recursive,
                separator,
            } => {
                let p = match prefix {
                    Some(v) => v.clone(),
                    _ => String::from(""),
                };
                let s = match separator {
                    Some(v) => v.clone(),
                    _ => String::from(""),
                };
                // flattening to array always sets an Object!
                let current = get_last(namespace, to);
                if let Some(v) = current.get_mut(id) {
                    if let Some(arr) = v.as_array_mut() {
                        if *index >= arr.len() {
                            arr.resize_with(*index + 1, Value::default);
                        }
                        let mut m = Map::new();
                        flatten(&s, &p, &field, &mut m, *recursive);
                        arr[*index] = Value::Object(m);
                    }
                } else {
                    // new array
                    let mut m = Map::new();
                    flatten(&s, &p, &field, &mut m, *recursive);
                    let mut new_arr = vec![Value::Null; *index];
                    new_arr.push(Value::Object(m));
                    current.insert(id.clone(), Value::Array(new_arr));
                }
            }
        }
        Ok(())
    }
}

#[inline]
fn flatten(sep: &str, id: &str, from: &Value, to: &mut Map<String, Value>, recursive: bool) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                let key = match id.len() {
                    0 => k.clone(),
                    _ => id.to_owned() + sep + k,
                };
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        if recursive {
                            flatten(sep, &key, v, to, recursive)
                        } else {
                            to.insert(key, v.clone());
                        }
                    }
                    _ => {
                        to.insert(key, v.clone());
                    }
                };
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = match id.len() {
                    0 => (i + 1).to_string(),
                    _ => id.to_owned() + sep + &(i + 1).to_string(),
                };
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        if recursive {
                            flatten(sep, &key, v, to, recursive)
                        } else {
                            to.insert(key, v.clone());
                        }
                    }
                    _ => {
                        to.insert(key, v.clone());
                    }
                };
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

impl Transform {
    pub fn parse(mapping: Mapping) -> Result<(Vec<Namespace>, Self)> {
        let mut from_namespace;
        let mut to_namespace;
        let mut is_flatten = false;
        let mut is_recursive = false;
        let mut flatten_prefix = None;
        let mut sep = None;

        let source = match mapping {
            Mapping::Direct { from, to } => {
                from_namespace = Namespace::parse(from)?;
                to_namespace = Namespace::parse(to)?;
                let field = from_namespace.pop().ok_or_else(|| {
                    Error::InvalidNamespace(String::from("No field defined for namespace"))
                })?;
                match field {
                    Namespace::Object { id } => Source::Direct(id),
                    Namespace::Array { id, index } => Source::DirectArray { id, index },
                }
            }
            Mapping::Constant { from, to } => {
                from_namespace = Vec::new();
                to_namespace = Namespace::parse(to)?;
                Source::Constant(from.clone())
            }
            Mapping::Flatten {
                from,
                to,
                prefix,
                recursive,
                separator,
            } => {
                is_flatten = true;
                is_recursive = recursive;
                flatten_prefix = prefix;
                sep = separator;
                from_namespace = Namespace::parse(from)?;
                to_namespace = Namespace::parse(to)?;
                let field = from_namespace.pop().ok_or_else(|| {
                    Error::InvalidNamespace(String::from("No field defined for namespace"))
                })?;
                match field {
                    Namespace::Object { id } => Source::Direct(id),
                    Namespace::Array { id, index } => Source::DirectArray { id, index },
                }
            }
        };
        let field = if is_flatten {
            // for flatten it's ok NOT to have a namespace
            to_namespace.pop().unwrap_or_else(|| Namespace::Object {
                id: String::from(""),
            })
        } else {
            to_namespace.pop().ok_or_else(|| {
                Error::InvalidNamespace(String::from("No field defined for namespace"))
            })?
        };

        let destination = match field {
            Namespace::Object { id } => {
                if is_flatten {
                    let p = match flatten_prefix {
                        Some(c) => Some(c.to_string()),
                        _ => None,
                    };
                    let s = match sep {
                        Some(c) => Some(c.to_string()),
                        _ => None,
                    };
                    let ident = match id.len() {
                        0 => None,
                        _ => Some(id),
                    };
                    Destination::FlattenDirect {
                        namespace: to_namespace,
                        id: ident,
                        prefix: p,
                        separator: s,
                        recursive: is_recursive,
                    }
                } else {
                    Destination::Direct {
                        namespace: to_namespace,
                        id,
                    }
                }
            }
            Namespace::Array { id, index } => {
                if is_flatten {
                    let p = match flatten_prefix {
                        Some(c) => Some(c.to_string()),
                        _ => None,
                    };
                    let s = match sep {
                        Some(c) => Some(c.to_string()),
                        _ => None,
                    };
                    Destination::FlattenArray {
                        namespace: to_namespace,
                        id,
                        prefix: p,
                        separator: s,
                        index,
                        recursive: is_recursive,
                    }
                } else {
                    Destination::DirectArray {
                        namespace: to_namespace,
                        id,
                        index,
                    }
                }
            }
        };
        Ok((
            from_namespace,
            Self {
                source,
                destination,
            },
        ))
    }
}

#[inline]
fn get_last<'a>(
    namespace: &[Namespace],
    mut current: &'a mut Map<String, Value>,
) -> &'a mut Map<String, Value> {
    for ns in namespace {
        match ns {
            Namespace::Object { id } => {
                current = current
                    .entry(id.clone()) // TODO: optimize later
                    .or_insert(Value::Object(Map::new()))
                    .as_object_mut()
                    .unwrap();
            }
            Namespace::Array { id, index } => {
                current = current
                    .entry(id.clone()) // TODO: optimize later
                    .or_insert(Value::Array(vec![Value::Null; *index]))
                    .as_object_mut()
                    .unwrap();
            }
        };
    }
    current
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Source {
    Direct(String),
    DirectArray { id: String, index: usize },
    Constant(Value),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Destination {
    Direct {
        namespace: Vec<Namespace>,
        id: String,
    },
    DirectArray {
        namespace: Vec<Namespace>,
        id: String,
        index: usize,
    },
    FlattenDirect {
        namespace: Vec<Namespace>,
        id: Option<String>,
        prefix: Option<String>,
        separator: Option<String>,
        recursive: bool,
    },
    FlattenArray {
        namespace: Vec<Namespace>,
        id: String,
        prefix: Option<String>,
        separator: Option<String>,
        index: usize,
        recursive: bool,
    },
}
