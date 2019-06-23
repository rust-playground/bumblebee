use crate::errors::Result;
use crate::namespace::Namespace;
use crate::rules::{FlattenOps, Mapping, Rule, Transform};
use crate::tree::{Arena, Node};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::fmt::Debug;

/// Mode defines the Transformers behaviour when encountering multiple element top level data such as
/// Array's. 99.99% of the time the default will suffice, however, there are times when you may wish to
/// transform from multiple in to a single which the One2One option allows.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Mode {
    One2One,
    Many2Many, // does OneToOne when input is NOT an array
               //    One2Many, // future functionality...maybe
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Many2Many
    }
}

/// TransformerBuilder is used to construct a new Transformer. Once a Transformer is build it is
/// immutable.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TransformerBuilder {
    root: Arena,
    mode: Mode,
}

impl TransformerBuilder {
    /// sets the mode for which the Transformer will operate.
    #[inline]
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// add allows any custom rule(s) to be added to the transformation beyond the built-in ones.
    #[inline]
    pub fn add<R>(mut self, namespace: &[Namespace], rule: R) -> Result<Self>
    where
        R: Rule + Debug + 'static,
    {
        self.root.add(namespace, rule);
        Ok(self)
    }

    /// adds mappings that may have been saved outside of this library for building UI's or other
    /// means of generically building transformations.
    #[inline]
    pub fn add_mappings(mut self, mappings: Vec<Mapping>) -> Result<Self> {
        for mapping in mappings {
            let (ns, rule) = Transform::parse(mapping)?;
            self = self.add(&ns, rule)?;
        }
        Ok(self)
    }

    /// adds a single mapping that may have been saved outside of this library for building UI's or
    /// other means of generically building transformations.
    #[inline]
    pub fn add_mapping(self, mapping: Mapping) -> Result<Self> {
        let (ns, rule) = Transform::parse(mapping)?;
        self.add(&ns, rule)
    }

    /// adds a constant value to a value on the output.
    #[inline]
    pub fn add_constant<'a, S, F>(self, from: F, to: S) -> Result<Self>
    where
        S: Into<Cow<'a, str>>,
        F: Into<Value>,
    {
        self.add_mapping(Mapping::Constant {
            from: from.into(),
            to: to.into(),
        })
    }

    /// adds a direct mapping from an existing value to a new value on the output.
    #[inline]
    pub fn add_direct<'a, S>(self, from: S, to: S) -> Result<Self>
    where
        S: Into<Cow<'a, str>>,
    {
        self.add_mapping(Mapping::Direct {
            from: from.into(),
            to: to.into(),
        })
    }

    /// adds a mapping which takes the existing value, either Object or Array, and flattens the data
    /// and places that at the desired output location.
    #[inline]
    pub fn add_flatten<'a, S>(self, from: S, to: S, options: FlattenOps) -> Result<Self>
    where
        S: Into<Cow<'a, str>>,
    {
        self.add_mapping(Mapping::Flatten {
            from: from.into(),
            to: to.into(),
            prefix: match options.prefix {
                Some(v) => Some(v.into()),
                None => None,
            },
            separator: match options.separator {
                Some(v) => Some(v.into()),
                None => None,
            },
            manipulation: match options.manipulation {
                Some(v) => Some(v.into()),
                None => None,
            },
            recursive: options.recursive,
        })
    }

    pub fn build(self) -> Result<Transformer> {
        Ok(Transformer {
            root: self.root,
            mode: self.mode,
        })
    }
}

/// Transformer is used to apply the transformation that's been built to any Serializable data.
#[derive(Debug, Serialize, Deserialize)]
pub struct Transformer {
    root: Arena,
    mode: Mode,
}

impl Transformer {
    /// applies the transformation to JSON withing a string
    #[inline]
    pub fn apply_from_str<'a, S>(&self, input: S) -> Result<Value>
    where
        S: Into<Cow<'a, str>>,
    {
        let results = transform(
            &self.mode,
            &self.root,
            self.root.tree.get(0).unwrap(), // root
            &serde_json::from_str(&input.into())?,
        )?;
        Ok(results)
    }

    /// applies the transformation to any serializable data and returns your desired structure.
    #[inline]
    pub fn apply_to<S, D>(&self, input: S) -> Result<D>
    where
        S: Serialize,
        D: DeserializeOwned,
    {
        let results = transform(
            &self.mode,
            &self.root,
            self.root.tree.get(0).unwrap(), // root
            &serde_json::to_value(input)?,
        )?;
        Ok(serde_json::from_value::<D>(results)?)
    }
}

#[inline]
fn transform(mode: &Mode, arena: &Arena, node: &Node, source: &Value) -> Result<Value> {
    match source {
        Value::Array(v) if mode == &Mode::Many2Many => {
            let mut new_arr = Vec::with_capacity(v.len());
            for value in v {
                let mut results = Map::new();
                transform_recursive(arena, node, value, &mut results)?;
                new_arr.push(Value::Object(results));
            }
            Ok(Value::Array(new_arr))
        }
        _ => {
            let mut results = Map::new();
            transform_recursive(arena, node, source, &mut results)?;
            Ok(Value::Object(results))
        }
    }
}

fn transform_recursive(
    arena: &Arena,
    node: &Node,
    source: &Value,
    dest: &mut Map<String, Value>,
) -> Result<()> {
    match node {
        Node::Object {
            rules, children, ..
        }
        | Node::Array {
            rules, children, ..
        } => {
            if let Some(rulz) = rules {
                for rule in rulz {
                    rule.apply(source, dest)?;
                }
            }
            if let Some((start, end)) = children {
                for idx in *start..=*end {
                    if let Some(n) = arena.tree.get(idx) {
                        match n {
                            Node::Object { id, .. } => {
                                // if we find the source value
                                if let Some(current_level) = source.get(id.as_str()) {
                                    transform_recursive(arena, n, current_level, dest)?;
                                }
                            }
                            Node::Array { id, index, .. } => {
                                // may be array of array already without id eg. arr[0][0]
                                if id != "" {
                                    if let Some(current_level) = source.get(id.as_str()) {
                                        if let Some(arr) = current_level.as_array() {
                                            if let Some(v) = arr.get(*index) {
                                                transform_recursive(arena, n, v, dest)?;
                                            }
                                        }
                                    }
                                } else if let Some(arr) = source.as_array() {
                                    if let Some(v) = arr.get(*index) {
                                        transform_recursive(arena, n, v, dest)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::StringManipulation;
    use serde::Deserialize;

    #[test]
    fn test_top_level() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("existing_key", "rename_from_existing_key")?
            .add_direct("my_array[0]", "used_to_be_array")?
            .add_constant(Value::String("consant_value".to_string()), "const")?
            .build()?;

        let input = r#"
            {
                "existing_key":"my_val1",
                "my_array":["idx_0_value"]
            }"#;
        let expected = r#"{"const":"consant_value","rename_from_existing_key":"my_val1","used_to_be_array":"idx_0_value"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[test]
    fn test_nested() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("nested.key1", "unnested_key1")?
            .add_direct("nested.nested.key2", "unnested_key2")?
            .add_direct("nested.arr[0].nested.key3", "unnested_key3")?
            .build()?;
        let input = r#"
                    {
                        "nested": {
                            "key1": "val1",
                            "nested": {
                                "key2": "val2"
                            },
                            "arr": [{
                                "nested": {
                                    "key3": "val3"
                                }
                            }]
                        }
                    }"#;
        let expected = r#"{"unnested_key1":"val1","unnested_key2":"val2","unnested_key3":"val3"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[test]
    fn test_nested_out_of_order_rules() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("nested.nested.key2", "nested_new.nested")?
            .add_direct("top", "nested_new.top")?
            .build()?;
        let input = r#"
                    {
                        "nested": {
                            "nested": {
                                "key2": "val2"
                            }
                        },
                        "top": "top_val"
                    }"#;
        let expected = r#"{"nested_new":{"nested":"val2","top":"top_val"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[test]
    fn test_full_objects() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("nested.nested.key2", "nested_new.nested")?
            .add_direct("top", "nested_new.top")?
            .build()?;
        let input = r#"
                    {
                        "nested": {
                            "nested": {
                                "key2": "val2"
                            }
                        },
                        "top": "top_val"
                    }"#;
        let expected = r#"{"nested_new":{"nested":"val2","top":"top_val"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[test]
    fn test_struct() -> Result<()> {
        #[derive(Debug, Serialize)]
        struct From {
            existing: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct To {
            new: String,
        }

        let trans = TransformerBuilder::default()
            .add_direct("existing", "new")?
            .build()?;

        let from = From {
            existing: String::from("existing_value"),
        };

        let expected = To {
            new: String::from("existing_value"),
        };
        let res: To = trans.apply_to(from)?;
        assert_eq!(expected, res);
        Ok(())
    }

    #[test]
    fn test_struct_enum() -> Result<()> {
        #[derive(Debug, Serialize)]
        struct From {
            existing: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct To {
            new: String,
        }

        let trans = TransformerBuilder::default()
            .add_direct("existing", "new")?
            .build()?;

        let from = From {
            existing: String::from("existing_value"),
        };

        let mut m = Map::new();
        m.insert(
            String::from("new"),
            Value::String(String::from("existing_value")),
        );
        let expected = Value::Object(m);
        let res: Value = trans.apply_to(from)?;
        assert_eq!(expected, res);
        Ok(())
    }

    #[test]
    fn test_array() -> Result<()> {
        let trans = TransformerBuilder::default()
            .mode(Mode::One2One)
            .add_direct("[0]", "new")?
            .build()?;
        let input = r#"[
                "test"
            ]"#;
        let expected = r#"{"new":"test"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[test]
    fn test_many_2_many() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("user_id", "id")?
            .add_direct("full_name", "name")?
            .build()?;
        let input = r#"[
                {"user_id":1,"full_name":"Dean Karn"},
                {"user_id":2, "full_name":"Joey Bloggs"}
            ]"#;
        let expected = r#"[{"id":1,"name":"Dean Karn"},{"id":2,"name":"Joey Bloggs"}]"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_flatten_direct() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "",
                FlattenOps {
                    recursive: false,
                    prefix: Some("flattened_"),
                    separator: None,
                    manipulation: None,
                },
            )?
            .build()?;
        let input = r#"{
                "nested":{
                    "key1":"value1",
                    "key2":"value2"
                }
            }"#;
        let expected = r#"{"flattened_key1":"value1","flattened_key2":"value2"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_flatten_direct_with_to() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "flattened",
                FlattenOps {
                    recursive: false,
                    prefix: Some("flattened_"),
                    separator: None,
                    manipulation: None,
                },
            )?
            .build()?;
        let input = r#"{
                "nested":{
                    "key1":"value1",
                    "key2":"value2"
                }
            }"#;
        let expected = r#"{"flattened":{"flattened_key1":"value1","flattened_key2":"value2"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }
    #[test]
    fn test_flatten_direct_with_to_no_profix() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten("nested", "flattened", FlattenOps::default())?
            .build()?;
        let input = r#"{
                "nested":{
                    "key1":"value1",
                    "key2":"value2"
                }
            }"#;
        let expected = r#"{"flattened":{"key1":"value1","key2":"value2"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_flatten_direct_recursive_with_to_no_prefix() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "",
                FlattenOps {
                    recursive: true,
                    prefix: None,
                    separator: Some("_"),
                    manipulation: None,
                },
            )?
            .build()?;
        let input = r#"{
            "nested":{
                "key1":"value1",
                "key2":{
                    "inner":"value2"
                }
            }
        }"#;
        let expected = r#"{"key1":"value1","key2_inner":"value2"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_flatten_direct_nonrecursive_with_to_no_prefix() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten("nested", "", FlattenOps::default())?
            .build()?;
        let input = r#"{
            "nested":{
                "key1":"value1",
                "key2":{
                    "inner":"value2"
                }
            }
        }"#;
        let expected = r#"{"key1":"value1","key2":{"inner":"value2"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_array_flatten() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "",
                FlattenOps {
                    recursive: false,
                    prefix: Some("new"),
                    separator: Some("_"),
                    manipulation: None,
                },
            )?
            .build()?;
        let input = r#"{
            "nested":[
                "value1",
                "value2",
                "value3"
            ]
        }"#;
        let expected = r#"{"new_1":"value1","new_2":"value2","new_3":"value3"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_array_flatten_to() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "flattened[1]",
                FlattenOps {
                    recursive: false,
                    prefix: Some("new"),
                    separator: Some("_"),
                    manipulation: None,
                },
            )?
            .build()?;
        let input = r#"{
            "nested":[
                "value1",
                "value2",
                "value3"
            ]
        }"#;
        let expected =
            r#"{"flattened":[null,{"new_1":"value1","new_2":"value2","new_3":"value3"}]}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }

    #[test]
    fn test_example() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_direct("user_id", "id")?
            .add_direct("full-name", "name")?
            .add_flatten(
                "nicknames",
                "",
                FlattenOps {
                    recursive: true,
                    prefix: Some("nickname"),
                    separator: Some("_"),
                    manipulation: None,
                },
            )?
            .add_direct("nested.inner.key", "prev_nested")?
            .add_direct("nested.my_arr[1]", "prev_arr")?
            .build()?;

        let input = r#"
            {
                "user_id":"111",
                "full-name":"Dean Karn",
                "nicknames":["Deano","Joey Bloggs"],
                "nested": {
                    "inner":{
                        "key":"value"
                    },
                    "my_arr":[null,"arr_value",null]
                }
            }"#;
        let expected = r#"{"id":"111","name":"Dean Karn","nickname_1":"Deano","nickname_2":"Joey Bloggs","prev_arr":"arr_value","prev_nested":"value"}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, serde_json::to_string(&res)?);
        Ok(())
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ManipDashRemover {}

    #[typetag::serde]
    impl StringManipulation for ManipDashRemover {
        fn apply(&self, input: &str) -> String {
            input.replace('-', "")
        }
    }

    #[test]
    fn test_flatten_direct_with_maipulation() -> Result<()> {
        let trans = TransformerBuilder::default()
            .add_flatten(
                "nested",
                "",
                FlattenOps {
                    manipulation: Some(Box::new(ManipDashRemover {})),
                    ..FlattenOps::default()
                },
            )?
            .build()?;
        let input = r#"{
            "nested":{
                "key-1":"value1",
                "key-2":{
                    "inner":"value2"
                }
            }
        }"#;
        let expected = r#"{"key1":"value1","key2":{"inner":"value2"}}"#;
        let res = trans.apply_from_str(input)?;
        assert_eq!(expected, res.to_string());
        Ok(())
    }
}
