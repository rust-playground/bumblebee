use crate::namespace::Namespace;
use crate::rules::Rule;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::mem;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Node {
    Object {
        id: String,
        children: Option<(usize, usize)>, // start + end tuple
        rules: Option<Vec<Box<dyn Rule>>>,
    },
    Array {
        index: usize,
        id: String,
        children: Option<(usize, usize)>, // start + end tuple
        rules: Option<Vec<Box<dyn Rule>>>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Arena {
    pub(crate) tree: Vec<Node>,
}

impl Default for Arena {
    fn default() -> Self {
        Arena {
            tree: vec![Node::Object {
                id: String::from(""),
                children: None,
                rules: None,
            }],
        }
    }
}

impl Arena {
    // TODO: investigate using Option for namespace below
    #[inline]
    pub fn add<R>(&mut self, namespace: &[Namespace], rule: R)
    where
        R: Rule + Debug + 'static,
    {
        // when top level there will be no namespaces
        let mut n = 0;
        'outer: for ns in namespace {
            // TODO: validate the children's namespace type matches the Namespace type

            match self.tree.get(n).unwrap() {
                Node::Object { children, .. } => {
                    if let Some((start, end)) = children.as_ref() {
                        for idx in *start..=*end {
                            match self.tree.get(idx).unwrap() {
                                Node::Object { id, .. } => {
                                    if id == ns.id() && ns.is_object() {
                                        n = idx;
                                        continue 'outer;
                                    }
                                }
                                Node::Array { index, id, .. } => {
                                    if id == ns.id()
                                        && ns.is_array()
                                        && index == ns.as_array().unwrap().1
                                    {
                                        n = idx;
                                        continue 'outer;
                                    }
                                }
                            }
                        }

                        let parent_idx = Some(n);
                        n = end + 1;
                        match ns {
                            Namespace::Object { id } => {
                                let new_node = Node::Object {
                                    id: id.clone(),
                                    children: None,
                                    rules: None,
                                };
                                self.reindex(parent_idx, n, new_node);
                            }
                            Namespace::Array { id, index } => {
                                let new_node = Node::Array {
                                    index: *index,
                                    id: id.clone(),
                                    children: None,
                                    rules: None,
                                };
                                self.reindex(parent_idx, n, new_node);
                            }
                        }
                        continue 'outer;
                    }

                    let parent_idx = Some(n);
                    n = self.tree.len();

                    match ns {
                        Namespace::Object { id } => {
                            let new_node = Node::Object {
                                id: id.clone(),
                                children: None,
                                rules: None,
                            };
                            self.reindex(parent_idx, n, new_node);
                        }
                        Namespace::Array { id, index } => {
                            let new_node = Node::Array {
                                index: *index,
                                id: id.clone(),
                                children: None,
                                rules: None,
                            };
                            self.reindex(parent_idx, n, new_node);
                        }
                    }
                }
                Node::Array {
                    // never be Node::Array for the root of the tree
                    children,
                    ..
                } => {
                    if let Some((start, end)) = children.as_ref() {
                        for idx in *start..=*end {
                            match self.tree.get(idx).unwrap() {
                                Node::Object { id, .. } => {
                                    if id == ns.id() && ns.is_object() {
                                        n = idx;
                                        continue 'outer;
                                    }
                                }
                                Node::Array { index, id, .. } => {
                                    if id == ns.id()
                                        && ns.is_array()
                                        && index == ns.as_array().unwrap().1
                                    {
                                        n = idx;
                                        continue 'outer;
                                    }
                                }
                            }
                        }

                        let parent_idx = Some(n);
                        n = end + 1;
                        match ns {
                            Namespace::Object { id } => {
                                let new_node = Node::Object {
                                    id: id.clone(),
                                    children: None,
                                    rules: None,
                                };
                                self.reindex(parent_idx, n, new_node);
                            }
                            Namespace::Array { id, index } => {
                                let new_node = Node::Array {
                                    index: *index,
                                    id: id.clone(),
                                    children: None,
                                    rules: None,
                                };
                                self.reindex(parent_idx, n, new_node);
                            }
                        }
                        continue 'outer;
                    }

                    let parent_idx = Some(n);
                    n = self.tree.len();
                    match ns {
                        Namespace::Object { id } => {
                            let new_node = Node::Object {
                                id: id.clone(),
                                children: None,
                                rules: None,
                            };
                            self.reindex(parent_idx, n, new_node);
                        }
                        Namespace::Array { id, index } => {
                            let new_node = Node::Array {
                                index: *index,
                                id: id.clone(),
                                children: None,
                                rules: None,
                            };
                            self.reindex(parent_idx, n, new_node);
                        }
                    }
                }
            }
        }
        let boxed_rule = Box::new(rule);
        let node = self.tree.get_mut(n).unwrap();
        match node {
            Node::Object { rules, .. } => match rules {
                Some(v) => v.push(boxed_rule),
                None => *rules = Some(vec![boxed_rule]),
            },
            Node::Array { rules, .. } => match rules {
                Some(v) => v.push(boxed_rule),
                None => *rules = Some(vec![boxed_rule]),
            },
        }
    }

    #[inline]
    fn reindex(&mut self, parent_idx: Option<usize>, index: usize, mut node: Node) {
        // loop over all nodes in tree
        for i in 0..self.tree.len() {
            // increase child count for any nodes that will be reindexed
            match self.tree.get_mut(i).unwrap() {
                Node::Object { children, .. } => {
                    if let Some((start, end)) = children {
                        if *start >= index {
                            *start += 1;
                            *end += 1;
                        }
                    }
                }
                Node::Array { children, .. } => {
                    if let Some((start, end)) = children {
                        if *start >= index {
                            *start += 1;
                            *end += 1;
                        }
                    }
                }
            }
            // if we're at the new nodes insertion point start reindexing
            if i >= index {
                node = mem::replace(&mut self.tree[i], node);
            }
        }
        self.tree.push(node);

        // increase or set the parent nodes child count to it's new range.
        if let Some(idx) = parent_idx {
            match self.tree.get_mut(idx).unwrap() {
                Node::Object { children, .. } => match children {
                    Some((_, end)) => {
                        *end += 1;
                    }
                    None => *children = Some((index, index)),
                },
                Node::Array { children, .. } => match children {
                    Some((_, end)) => {
                        *end += 1;
                    }
                    None => *children = Some((index, index)),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Result;
    use serde::{Deserialize, Serialize};
    use serde_json::{Map, Value};

    #[derive(Debug, Serialize, Deserialize)]
    struct MyRule {}

    #[typetag::serde]
    impl Rule for MyRule {
        fn apply(&self, from: &Value, _to: &mut Map<String, Value>) -> Result<()> {
            dbg!(from);
            Ok(())
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MyRule2 {}

    #[typetag::serde]
    impl Rule for MyRule2 {
        fn apply(&self, from: &Value, _to: &mut Map<String, Value>) -> Result<()> {
            dbg!(from);
            Ok(())
        }
    }

    #[test]
    fn test_simple() {
        let rule = MyRule {};
        let namespace = vec![];

        let mut arena = Arena::default();
        arena.add(&namespace, rule);

        let rule2 = MyRule2 {};
        arena.add(&namespace, rule2);

        // add a nested value
        let rule3 = MyRule {};
        let embedded = vec![Namespace::Object {
            id: String::from("embedded"),
        }];
        arena.add(&embedded, rule3);

        // add a nested value
        let rule4 = MyRule2 {};
        arena.add(&embedded, rule4);

        let rule5 = MyRule {};
        let embedded = vec![Namespace::Object {
            id: String::from("embedded2"),
        }];
        arena.add(&embedded, rule5);

        let rule6 = MyRule {};
        let embedded = vec![
            Namespace::Object {
                id: String::from("embedded"),
            },
            Namespace::Object {
                id: String::from("injected-child"),
            },
        ];
        arena.add(&embedded, rule6);

        let rule7 = MyRule {};
        let embedded = vec![
            Namespace::Object {
                id: String::from("embedded"),
            },
            Namespace::Object {
                id: String::from("injected-child2"),
            },
        ];
        arena.add(&embedded, rule7);

        let rule8 = MyRule {};
        let embedded = vec![
            Namespace::Object {
                id: String::from("embedded2"),
            },
            Namespace::Object {
                id: String::from("embedded2-injected-child"),
            },
        ];
        arena.add(&embedded, rule8);

        let rule9 = MyRule {};
        let embedded = vec![
            Namespace::Object {
                id: String::from("embedded"),
            },
            Namespace::Object {
                id: String::from("injected-child3"),
            },
        ];
        arena.add(&embedded, rule9);

        let rule10 = MyRule {};
        let embedded = vec![
            Namespace::Object {
                id: String::from("embedded2"),
            },
            Namespace::Object {
                id: String::from("embedded2-injected-child2"),
            },
        ];
        arena.add(&embedded, rule10);

        // add a nested value
        let rule11 = MyRule {};
        let embedded = vec![Namespace::Object {
            id: String::from("injected-embedded-after"),
        }];
        arena.add(&embedded, rule11);

        let tree = vec![
            Node::Object {
                id: "".to_string(),
                children: Some((1, 3)),
                rules: Some(vec![Box::new(MyRule {}), Box::new(MyRule2 {})]),
            },
            Node::Object {
                id: "embedded".to_string(),
                children: Some((4, 6)),
                rules: Some(vec![Box::new(MyRule {}), Box::new(MyRule2 {})]),
            },
            Node::Object {
                id: "embedded2".to_string(),
                children: Some((7, 8)),
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "injected-embedded-after".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "injected-child".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "injected-child2".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "injected-child3".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "embedded2-injected-child".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
            Node::Object {
                id: "embedded2-injected-child2".to_string(),
                children: None,
                rules: Some(vec![Box::new(MyRule {})]),
            },
        ];
        let expected = Arena { tree };
        assert_eq!(format!("{:?}", expected), format!("{:?}", arena));
    }
}
