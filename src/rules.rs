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

#[typetag::serde]
pub trait StringManipulation: Debug {
    fn apply(&self, input: &str) -> String;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FlattenOps<'a> {
    pub recursive: bool,
    pub prefix: Option<&'a str>,
    pub separator: Option<&'a str>,
    pub manipulation: Option<Box<dyn StringManipulation>>,
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
        manipulation: Option<Box<dyn StringManipulation>>,
        recursive: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Transform {
    source: Source,
    destination: Destination,
}

#[typetag::serde]
impl Rule for Transform {
    fn apply(&self, from: &Value, to: &mut Map<String, Value>) -> Result<()> {
        let field = match &self.source {
            Source::Direct(id) => match from {
                Value::Object(obj) => obj.get(id).unwrap_or(&Value::Null).clone(),
                _ => Value::Null,
            },
            Source::DirectArray { id, index } => match from {
                Value::Object(v) => match v.get(id) {
                    Some(arr) => arr.get(index).unwrap_or(&Value::Null).clone(),
                    _ => Value::Null,
                },
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
                match current.get_mut(id) {
                    Some(v) => {
                        if let Some(arr) = v.as_array_mut() {
                            if *index >= arr.len() {
                                arr.resize_with(*index + 1, Value::default);
                            }
                            arr[*index] = field;
                        }
                    }
                    _ => {
                        let mut new_arr = vec![Value::Null; *index];
                        new_arr.push(field);
                        current.insert(id.clone(), Value::Array(new_arr));
                    }
                }
            }
            Destination::FlattenDirect {
                id,
                namespace,
                recursive,
                prefix,
                manipulation,
                separator,
            } => match id {
                Some(id) => {
                    let mut m = Map::new();
                    flatten(
                        &manipulation,
                        &separator,
                        &prefix,
                        &field,
                        &mut m,
                        *recursive,
                    );
                    get_last(namespace, to).insert(id.clone(), Value::Object(m));
                }
                None => {
                    flatten(
                        &manipulation,
                        &separator,
                        &prefix,
                        &field,
                        get_last(namespace, to),
                        *recursive,
                    );
                }
            },
            Destination::FlattenArray {
                id,
                namespace,
                prefix,
                manipulation,
                index,
                recursive,
                separator,
            } => {
                let current = get_last(namespace, to);
                match current.get_mut(id) {
                    Some(v) => {
                        if let Some(arr) = v.as_array_mut() {
                            if *index >= arr.len() {
                                arr.resize_with(*index + 1, Value::default);
                            }
                            let mut m = Map::new();
                            flatten(
                                &manipulation,
                                &separator,
                                &prefix,
                                &field,
                                &mut m,
                                *recursive,
                            );
                            arr[*index] = Value::Object(m);
                        }
                    }
                    _ => {
                        let mut m = Map::new();
                        flatten(
                            &manipulation,
                            &separator,
                            &prefix,
                            &field,
                            &mut m,
                            *recursive,
                        );
                        let mut new_arr = vec![Value::Null; *index];
                        new_arr.push(Value::Object(m));
                        current.insert(id.clone(), Value::Array(new_arr));
                    }
                }
            }
        }
        Ok(())
    }
}

#[inline]
fn flatten_recursive_no_id(sep: &str, id: &str, from: &Value, to: &mut Map<String, Value>) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id(sep, k, v, to),
                    _ => {
                        to.insert(k.clone(), v.clone());
                    }
                };
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        flatten_recursive_with_id(sep, &(i + 1).to_string(), v, to)
                    }
                    _ => {
                        to.insert((i + 1).to_string(), v.clone());
                    }
                };
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten_recursive_no_id_manipulation(
    manipulation: &dyn StringManipulation,
    sep: &str,
    id: &str,
    from: &Value,
    to: &mut Map<String, Value>,
) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id_manipulation(
                        manipulation,
                        sep,
                        &manipulation.apply(k),
                        v,
                        to,
                    ),
                    _ => {
                        to.insert(manipulation.apply(k), v.clone());
                    }
                };
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id_manipulation(
                        manipulation,
                        sep,
                        &(i + 1).to_string(),
                        v,
                        to,
                    ),
                    _ => {
                        to.insert((i + 1).to_string(), v.clone());
                    }
                };
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

fn flatten_recursive_with_id(sep: &str, id: &str, from: &Value, to: &mut Map<String, Value>) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                match v {
                    Value::Object(_) | Value::Array(_) => {
                        flatten_recursive_with_id(sep, &(id.to_owned() + sep + k), v, to)
                    }
                    _ => {
                        to.insert(id.to_owned() + sep + k, v.clone());
                    }
                };
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id(
                        sep,
                        &(id.to_owned() + sep + &(i + 1).to_string()),
                        v,
                        to,
                    ),
                    _ => {
                        to.insert(id.to_owned() + sep + &(i + 1).to_string(), v.clone());
                    }
                };
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

fn flatten_recursive_with_id_manipulation(
    manipulation: &dyn StringManipulation,
    sep: &str,
    id: &str,
    from: &Value,
    to: &mut Map<String, Value>,
) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id(
                        sep,
                        &(id.to_owned() + sep + &manipulation.apply(k)),
                        v,
                        to,
                    ),
                    _ => {
                        to.insert(id.to_owned() + sep + &manipulation.apply(k), v.clone());
                    }
                };
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                match v {
                    Value::Object(_) | Value::Array(_) => flatten_recursive_with_id(
                        sep,
                        &(id.to_owned() + sep + &(i + 1).to_string()),
                        v,
                        to,
                    ),
                    _ => {
                        to.insert(id.to_owned() + sep + &(i + 1).to_string(), v.clone());
                    }
                };
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten_single_level_no_id(id: &str, from: &Value, to: &mut Map<String, Value>) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                to.insert(k.clone(), v.clone());
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                to.insert((i + 1).to_string(), v.clone());
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten_single_level_with_id(sep: &str, id: &str, from: &Value, to: &mut Map<String, Value>) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                to.insert(id.to_owned() + sep + k, v.clone());
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                to.insert(id.to_owned() + sep + &(i + 1).to_string(), v.clone());
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten_single_level_no_id_manipulation(
    manipulation: &dyn StringManipulation,
    id: &str,
    from: &Value,
    to: &mut Map<String, Value>,
) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                to.insert(manipulation.apply(k), v.clone());
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                to.insert((i + 1).to_string(), v.clone());
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten_single_level_with_id_manipulation(
    manipulation: &dyn StringManipulation,
    sep: &str,
    id: &str,
    from: &Value,
    to: &mut Map<String, Value>,
) {
    match from {
        Value::Object(m) => {
            for (k, v) in m {
                to.insert(id.to_owned() + sep + &manipulation.apply(k), v.clone());
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                to.insert(id.to_owned() + sep + &(i + 1).to_string(), v.clone());
            }
        }
        _ => {
            to.insert(id.to_owned(), from.clone());
        }
    }
}

#[inline]
fn flatten(
    manipulation: &Option<Box<dyn StringManipulation>>,
    sep: &str,
    id: &str,
    from: &Value,
    to: &mut Map<String, Value>,
    recursive: bool,
) {
    if recursive {
        match manipulation {
            Some(man) => match id.len() {
                0 => flatten_recursive_no_id_manipulation(man.as_ref(), sep, id, from, to),
                _ => flatten_recursive_with_id_manipulation(man.as_ref(), sep, id, from, to),
            },
            None => match id.len() {
                0 => flatten_recursive_no_id(sep, id, from, to),
                _ => flatten_recursive_with_id(sep, id, from, to),
            },
        };
    } else {
        match manipulation {
            Some(man) => match id.len() {
                0 => flatten_single_level_no_id_manipulation(man.as_ref(), id, from, to),
                _ => flatten_single_level_with_id_manipulation(man.as_ref(), sep, id, from, to),
            },
            None => match id.len() {
                0 => flatten_single_level_no_id(id, from, to),
                _ => flatten_single_level_with_id(sep, id, from, to),
            },
        };
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
        let mut manip = None;

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
                manipulation,
                recursive,
                separator,
            } => {
                is_flatten = true;
                is_recursive = recursive;
                flatten_prefix = prefix;
                sep = separator;
                manip = manipulation;
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
                    Destination::FlattenDirect {
                        namespace: to_namespace,
                        id: match id.len() {
                            0 => None,
                            _ => Some(id),
                        },
                        prefix: match flatten_prefix {
                            Some(c) => c.to_string(),
                            _ => String::from(""),
                        },
                        separator: match sep {
                            Some(c) => c.to_string(),
                            _ => String::from(""),
                        },
                        manipulation: manip,
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
                    Destination::FlattenArray {
                        namespace: to_namespace,
                        id,
                        prefix: match flatten_prefix {
                            Some(c) => c.to_string(),
                            _ => String::from(""),
                        },
                        separator: match sep {
                            Some(c) => c.to_string(),
                            _ => String::from(""),
                        },
                        index,
                        manipulation: manip,
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
                    .entry(id.clone())
                    .or_insert(Value::Object(Map::new()))
                    .as_object_mut()
                    .unwrap();
            }
            Namespace::Array { id, index } => {
                current = current
                    .entry(id.clone())
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

#[derive(Debug, Serialize, Deserialize)]
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
        prefix: String,
        separator: String,
        manipulation: Option<Box<dyn StringManipulation>>,
        recursive: bool,
    },
    FlattenArray {
        namespace: Vec<Namespace>,
        id: String,
        prefix: String,
        separator: String,
        manipulation: Option<Box<dyn StringManipulation>>,
        index: usize,
        recursive: bool,
    },
}
