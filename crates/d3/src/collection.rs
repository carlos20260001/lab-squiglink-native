//! A port of **d3-collection**, from the JavaScript of the same name.
//!
//! Translated from `d3-collection/src/`, version 1.0.7, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! The JavaScript's own map and set are objects with a prefix on every
//! key, so that a key cannot be one of the object's own; the port needs
//! no prefix, for it has no prototype to be confused with, and the keys
//! are strings, as they are there.

pub mod nest;

use std::collections::BTreeMap;

/// A map, which holds a value under a key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Map<V> {
    entries: Vec<(String, V)>,
}

impl<V> Map<V> {
    /// The map the JavaScript's own makes.
    pub fn new() -> Self {
        Map {
            entries: Vec::new(),
        }
    }

    /// Whether the map holds a key.
    pub fn has(&self, key: &str) -> bool {
        self.entries.iter().any(|(name, _)| name == key)
    }

    /// The value under a key.
    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The value under a key, which is set.
    pub fn set(&mut self, key: &str, value: V) -> &mut Self {
        match self.entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key.to_owned(), value)),
        }
        self
    }

    /// Whether a key is taken out of the map.
    pub fn remove(&mut self, key: &str) -> bool {
        match self.entries.iter().position(|(name, _)| name == key) {
            Some(at) => {
                self.entries.remove(at);
                true
            }
            None => false,
        }
    }

    /// The map is emptied.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The keys of the map, in the order they were put in.
    pub fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(key, _)| key.clone()).collect()
    }

    /// The values of the map, in the order they were put in.
    pub fn values(&self) -> Vec<&V> {
        self.entries.iter().map(|(_, value)| value).collect()
    }

    /// The entries of the map, in the order they were put in.
    pub fn entries(&self) -> Vec<(String, &V)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.clone(), value))
            .collect()
    }

    /// The count of the keys.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Whether the map holds nothing.
    pub fn empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every value of the map, with its key.
    pub fn each<F: FnMut(&V, &str)>(&self, mut function: F) {
        for (key, value) in &self.entries {
            function(value, key);
        }
    }
}

/// The JavaScript's `map`, which makes a map of an array or an object.
///
/// The function, which reads the key of a value, is given the value, its
/// place in the array, and the array itself.
pub fn map<V: Clone, F: Fn(&V, usize) -> String>(object: &[V], function: Option<F>) -> Map<V> {
    let mut map = Map::new();
    for (index, value) in object.iter().enumerate() {
        let key = match &function {
            Some(function) => function(value, index),
            None => index.to_string(),
        };
        map.set(&key, value.clone());
    }
    map
}

/// The JavaScript's `map`, which makes a map of an object.
pub fn map_of_object<V: Clone>(object: &BTreeMap<String, V>) -> Map<V> {
    let mut map = Map::new();
    for (key, value) in object {
        map.set(key, value.clone());
    }
    map
}

/// A set, which holds values without a key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Set {
    values: Vec<String>,
}

impl Set {
    /// The set the JavaScript's own makes.
    pub fn new() -> Self {
        Set { values: Vec::new() }
    }

    /// Whether the set holds a value.
    pub fn has(&self, value: &str) -> bool {
        self.values.iter().any(|name| name == value)
    }

    /// A value is put into the set.
    pub fn add(&mut self, value: &str) -> &mut Self {
        if !self.has(value) {
            self.values.push(value.to_owned());
        }
        self
    }

    /// Whether a value is taken out of the set.
    pub fn remove(&mut self, value: &str) -> bool {
        match self.values.iter().position(|name| name == value) {
            Some(at) => {
                self.values.remove(at);
                true
            }
            None => false,
        }
    }

    /// The set is emptied.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// The values of the set, in the order they were put in.
    pub fn values(&self) -> Vec<String> {
        self.values.clone()
    }

    /// The count of the values.
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// Whether the set holds nothing.
    pub fn empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Every value of the set.
    pub fn each<F: FnMut(&str)>(&self, mut function: F) {
        for value in &self.values {
            function(value);
        }
    }
}

/// The JavaScript's `set`, which makes a set of an array.
///
/// The function, which reads the value, is given the value, its place in
/// the array, and the array itself.
pub fn set<V: std::fmt::Display, F: Fn(&V, usize) -> String>(
    object: &[V],
    function: Option<F>,
) -> Set {
    let mut set = Set::new();
    for (index, value) in object.iter().enumerate() {
        let value = match &function {
            Some(function) => function(value, index),
            None => value.to_string(),
        };
        set.add(&value);
    }
    set
}

/// The JavaScript's `keys`, which are the keys of a map.
pub fn keys<V>(map: &Map<V>) -> Vec<String> {
    map.keys()
}

/// The JavaScript's `values`, which are the values of a map.
pub fn values<V>(map: &Map<V>) -> Vec<&V> {
    map.values()
}

/// The JavaScript's `entries`, which are the entries of a map.
pub fn entries<V>(map: &Map<V>) -> Vec<(String, &V)> {
    map.entries()
}
