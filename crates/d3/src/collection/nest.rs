//! The JavaScript's own nesting of values.
//!
//! Translated from `d3-collection/src/nest.js`, version 1.0.7, of d3
//! 5.16.0.

use crate::collection::Map;

/// The order of two keys ✓.
type Order = fn(&str, &str) -> std::cmp::Ordering;

/// The order of two values ✓.
type ValueOrder<V> = fn(&V, &V) -> std::cmp::Ordering;

/// A node of a nest, which is either a nest of them or a list.
#[derive(Clone, Debug, PartialEq)]
pub enum Node<V> {
    /// The values under their keys.
    Nest(Vec<(String, Node<V>)>),
    /// The list of the values, which is the end of the nest.
    List(Vec<V>),
}

/// The entries of a nest: a key, and what is under it.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry<V> {
    /// The entries under a key.
    Level(String, Vec<Entry<V>>),
    /// The values under a key, which is the end of the nest.
    Leaf(String, Vec<V>),
}

/// The JavaScript's `nest`, which groups values by their keys.
///
/// The keys are the functions that read the key of a value ✓ and they
/// are put in the order they are given ✓.
pub struct Nest<V> {
    keys: Vec<fn(&V) -> String>,
    sort_keys: Vec<Option<Order>>,
    sort_values: Option<ValueOrder<V>>,
    rollup: Option<fn(Vec<V>) -> Vec<V>>,
}

impl<V> Default for Nest<V> {
    fn default() -> Self {
        Nest::new()
    }
}

impl<V> Nest<V> {
    /// The nest the JavaScript's own makes.
    pub fn new() -> Self {
        Nest {
            keys: Vec::new(),
            sort_keys: Vec::new(),
            sort_values: None,
            rollup: None,
        }
    }

    /// A key is put to the nest ✓.
    pub fn key(mut self, key: fn(&V) -> String) -> Self {
        self.keys.push(key);
        self
    }

    /// The keys of a level are put in an order ✓.
    pub fn sort_keys(mut self, order: fn(&str, &str) -> std::cmp::Ordering) -> Self {
        let at = self.keys.len().saturating_sub(1);
        if self.sort_keys.len() <= at {
            self.sort_keys.resize(at + 1, None);
        }
        self.sort_keys[at] = Some(order);
        self
    }

    /// The values of a level are put in an order ✓.
    pub fn sort_values(mut self, order: fn(&V, &V) -> std::cmp::Ordering) -> Self {
        self.sort_values = Some(order);
        self
    }

    /// The values of a level are gathered into one ✓.
    pub fn rollup(mut self, function: fn(Vec<V>) -> Vec<V>) -> Self {
        self.rollup = Some(function);
        self
    }

    /// The values, gathered into a nest of maps ✓.
    pub fn map(&self, array: &[V]) -> Map<Node<V>>
    where
        V: Clone,
    {
        to_map(&self.apply(array, 0))
    }

    /// The values, gathered into a nest of objects ✓.
    pub fn object(&self, array: &[V]) -> Vec<(String, Node<V>)>
    where
        V: Clone,
    {
        match self.apply(array, 0) {
            Node::Nest(entries) => entries,
            Node::List(values) => values
                .into_iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), Node::List(vec![value])))
                .collect(),
        }
    }

    /// The values, gathered into a nest, and the entries of it ✓.
    pub fn entries(&self, array: &[V]) -> Vec<Entry<V>>
    where
        V: Clone,
    {
        self.entries_of(&self.apply(array, 0), 0)
    }

    /// The JavaScript's `apply`, which gathers the values ✓.
    fn apply(&self, array: &[V], depth: usize) -> Node<V>
    where
        V: Clone,
    {
        if depth >= self.keys.len() {
            let mut values = array.to_vec();
            if let Some(order) = self.sort_values {
                values.sort_by(order);
            }
            return match self.rollup {
                Some(rollup) => Node::List(rollup(values)),
                None => Node::List(values),
            };
        }
        let key = self.keys[depth];
        let mut by_key: Vec<(String, Vec<V>)> = Vec::new();
        for value in array {
            let key_value = key(value);
            match by_key.iter_mut().find(|(name, _)| *name == key_value) {
                Some((_, values)) => values.push(value.clone()),
                None => by_key.push((key_value, vec![value.clone()])),
            }
        }
        let mut result = Vec::new();
        for (key, values) in by_key {
            result.push((key, self.apply(&values, depth + 1)));
        }
        Node::Nest(result)
    }

    /// The JavaScript's `entries`, which are the entries of a nest ✓.
    fn entries_of(&self, node: &Node<V>, depth: usize) -> Vec<Entry<V>>
    where
        V: Clone,
    {
        let depth = depth + 1;
        if depth > self.keys.len() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        match node {
            Node::Nest(values) => {
                for (key, value) in values {
                    entries.push(self.entry_of(key, value, depth));
                }
            }
            Node::List(values) => {
                for value in values {
                    entries.push(Entry::Leaf(String::new(), vec![value.clone()]));
                }
            }
        }
        if let Some(Some(order)) = self.sort_keys.get(depth - 1) {
            entries.sort_by(|a, b| order(key_of(a), key_of(b)));
        }
        entries
    }

    /// The entry of a key, which is a level or the values ✓.
    fn entry_of(&self, key: &str, node: &Node<V>, _depth: usize) -> Entry<V>
    where
        V: Clone,
    {
        match node {
            Node::Nest(values) => Entry::Level(
                key.to_owned(),
                values
                    .iter()
                    .map(|(key, value)| self.entry_of(key, value, 0))
                    .collect(),
            ),
            Node::List(values) => Entry::Leaf(key.to_owned(), values.clone()),
        }
    }
}

impl<V> Entry<V> {
    /// The key of the entry ✓.
    pub fn key(&self) -> &str {
        match self {
            Entry::Level(key, _) => key,
            Entry::Leaf(key, _) => key,
        }
    }

    /// The entries under the key ✓.
    pub fn values(&self) -> Vec<&Entry<V>> {
        match self {
            Entry::Level(_, values) => values.iter().collect(),
            Entry::Leaf(_, _) => Vec::new(),
        }
    }
}

/// The key of an entry ✓.
fn key_of<V>(entry: &Entry<V>) -> &str {
    match entry {
        Entry::Level(key, _) => key,
        Entry::Leaf(key, _) => key,
    }
}

/// The nest of a node, as a map ✓.
fn to_map<V: Clone>(node: &Node<V>) -> Map<Node<V>> {
    let mut map = Map::new();
    match node {
        Node::Nest(entries) => {
            for (key, value) in entries {
                map.set(key, value.clone());
            }
        }
        Node::List(values) => {
            for (index, value) in values.iter().enumerate() {
                map.set(&index.to_string(), Node::List(vec![value.clone()]));
            }
        }
    }
    map
}
