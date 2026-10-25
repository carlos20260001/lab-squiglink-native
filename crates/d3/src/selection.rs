//! A faithful port of **d3-selection**, from the JavaScript of the same name.
//!
//! Translated from `d3-selection/src/`, version 1.4.1, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! # What the JavaScript is
//!
//! Selections allow powerful data-driven transformation of the document object
//! model (DOM): setting attributes, styles, properties, HTML or text content,
//! registering event listeners, and adding, removing or sorting elements.
//!
//! The data-join allows declarative manipulation of elements: entering, updating,
//! and exiting elements are clearly separated, enabling concise descriptions of
//! how data should map to visual elements.
//!
//! Selections are generic over any type implementing the [`Dom`] trait, and default
//! to the built-in [`SimpleNode`] in-memory DOM.

use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use crate::dom::{create_document, create_element, Dom, SimpleNode};

// ---------------------------------------------------------------------------
// Thread-local event context
// ---------------------------------------------------------------------------

thread_local! {
    static CURRENT_EVENT: RefCell<Option<Event>> = const { RefCell::new(None) };
}

/// An event in the JavaScript's sense (`d3.event`).
#[derive(Clone, Debug, Default)]
pub struct Event {
    /// The event type (e.g. "mousedown", "touchstart").
    pub event_type: String,
    /// The client X coordinate in viewport space.
    pub client_x: f64,
    /// The client Y coordinate in viewport space.
    pub client_y: f64,
    /// Whether Control key was pressed.
    pub ctrl_key: bool,
    /// Whether Shift key was pressed.
    pub shift_key: bool,
    /// Whether Alt key was pressed.
    pub alt_key: bool,
    /// Whether Meta key was pressed.
    pub meta_key: bool,
    /// Mouse button pressed (0 = primary, 1 = auxiliary, 2 = secondary).
    pub button: i32,
    /// Mouse buttons pressed bitmask.
    pub buttons: i32,
    /// Touch identifier.
    pub identifier: i32,
    /// Current touches.
    pub touches: Vec<Touch>,
    /// Changed touches.
    pub changed_touches: Vec<Touch>,
    /// The underlying or previous source event.
    pub source_event: Option<Box<Event>>,
}

/// A touch point within a touch event.
#[derive(Clone, Debug, Default)]
pub struct Touch {
    /// Unique identifier for this touch point.
    pub identifier: i32,
    /// Client X coordinate in viewport space.
    pub client_x: f64,
    /// Client Y coordinate in viewport space.
    pub client_y: f64,
}

/// The JavaScript's `d3.event`, returning the event currently being handled.
pub fn event() -> Option<Event> {
    CURRENT_EVENT.with(|e| e.borrow().clone())
}

/// Alias for [`event`].
pub fn current_event() -> Option<Event> {
    event()
}

/// The JavaScript's `customEvent(event1, listener, that, args)`.
pub fn custom_event<R, F: FnOnce() -> R>(mut new_event: Event, f: F) -> R {
    CURRENT_EVENT.with(|cell| {
        let prev = cell.borrow_mut().take();
        new_event.source_event = prev.clone().map(Box::new);
        *cell.borrow_mut() = Some(new_event);
        let res = f();
        *cell.borrow_mut() = prev;
        res
    })
}

/// The JavaScript's `sourceEvent()`, which walks up through nested `sourceEvent`s.
pub fn source_event() -> Option<Event> {
    let mut curr = event()?;
    while let Some(src) = curr.source_event.take() {
        curr = *src;
    }
    Some(curr)
}

// ---------------------------------------------------------------------------
// Namespaces
// ---------------------------------------------------------------------------

/// Well-known XML namespace URIs.
pub const NAMESPACE_SVG: &str = "http://www.w3.org/2000/svg";
/// XHTML namespace URI.
pub const NAMESPACE_XHTML: &str = "http://www.w3.org/1999/xhtml";
/// XLink namespace URI.
pub const NAMESPACE_XLINK: &str = "http://www.w3.org/1999/xlink";
/// XML namespace URI.
pub const NAMESPACE_XML: &str = "http://www.w3.org/XML/1998/namespace";
/// XMLNS namespace URI.
pub const NAMESPACE_XMLNS: &str = "http://www.w3.org/2000/xmlns/";

/// The JavaScript's `namespaces` map.
pub fn namespaces() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("svg", NAMESPACE_SVG);
    map.insert("xhtml", NAMESPACE_XHTML);
    map.insert("xlink", NAMESPACE_XLINK);
    map.insert("xml", NAMESPACE_XML);
    map.insert("xmlns", NAMESPACE_XMLNS);
    map
}

/// A qualified namespace pair `(space, local)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Namespace {
    /// The namespace URI (e.g. `http://www.w3.org/2000/svg`).
    pub space: Option<String>,
    /// The local element or attribute name.
    pub local: String,
}

/// The JavaScript's `d3.namespace(name)`.
pub fn namespace(name: &str) -> Namespace {
    if let Some(i) = name.find(':') {
        let prefix = &name[..i];
        let local = &name[i + 1..];
        if prefix != "xmlns" {
            let ns_map = namespaces();
            if let Some(&space) = ns_map.get(prefix) {
                return Namespace {
                    space: Some(space.to_string()),
                    local: local.to_string(),
                };
            }
        }
    }
    Namespace {
        space: None,
        local: name.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Nodes in a Selection: materialized or enter placeholder
// ---------------------------------------------------------------------------

/// A placeholder node for entering data that has not yet been materialized.
#[derive(Clone, Debug)]
pub struct EnterNode<N: Dom> {
    /// The parent node where this enter node will be inserted.
    pub parent: Option<N>,
    /// The next sibling before which this node should be inserted.
    pub next: Option<N>,
    /// The datum bound to this enter node.
    pub datum: Option<Rc<dyn Any>>,
}

/// A node within a selection group: either a real DOM node or an enter node.
#[derive(Clone, Debug)]
pub enum SelectedNode<N: Dom> {
    /// A materialized DOM node.
    Node(N),
    /// An enter placeholder node with bound datum.
    Enter(EnterNode<N>),
}

impl<N: Dom> SelectedNode<N> {
    /// Return the underlying DOM node if this is a materialized node.
    pub fn node(&self) -> Option<&N> {
        match self {
            SelectedNode::Node(n) => Some(n),
            SelectedNode::Enter(_) => None,
        }
    }

    /// Return the datum associated with this node or enter placeholder.
    pub fn datum(&self) -> Option<Rc<dyn Any>> {
        match self {
            SelectedNode::Node(n) => n.get_any("__data__"),
            SelectedNode::Enter(e) => e.datum.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// The Selection struct
// ---------------------------------------------------------------------------

/// The JavaScript's `d3.selection`.
///
/// A selection is an array of groups, where each group is an array of elements.
/// It also carries parent nodes for each group, and optional enter/exit groups
/// from data joins.
#[derive(Clone, Debug)]
pub struct Selection<N: Dom = SimpleNode> {
    /// The groups of elements in this selection.
    pub _groups: Vec<Vec<Option<SelectedNode<N>>>>,
    /// The parent node for each group.
    pub _parents: Vec<Option<N>>,
    /// Enter nodes from a data join.
    pub _enter: Option<Vec<Vec<Option<EnterNode<N>>>>>,
    /// Exit nodes from a data join.
    pub _exit: Option<Vec<Vec<Option<N>>>>,
}

impl<N: Dom> Selection<N> {
    /// Create a new selection from groups and parents.
    pub fn new(groups: Vec<Vec<Option<SelectedNode<N>>>>, parents: Vec<Option<N>>) -> Self {
        Selection {
            _groups: groups,
            _parents: parents,
            _enter: None,
            _exit: None,
        }
    }

    /// Create a selection wrapping a single DOM node.
    pub fn from_node(node: N) -> Self {
        Selection::new(vec![vec![Some(SelectedNode::Node(node))]], vec![None])
    }

    /// Create a selection wrapping an array of DOM nodes.
    pub fn from_nodes(nodes: Vec<N>) -> Self {
        let group = nodes
            .into_iter()
            .map(|n| Some(SelectedNode::Node(n)))
            .collect();
        Selection::new(vec![group], vec![None])
    }

    // -----------------------------------------------------------------------
    // Sub-selection: select, selectAll, filter
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.select(selector)`.
    ///
    /// For each selected element, selects the first descendant matching the
    /// selector. If the selector is a string, it evaluates `querySelector`.
    pub fn select(&self, selector: &str) -> Self {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::with_capacity(group.len());
            for node_opt in group {
                let subnode = if let Some(SelectedNode::Node(node)) = node_opt {
                    node.query_selector(selector).map(|child| {
                        // Data propagation: if node has __data__, pass to child
                        if let Some(d) = node.get_any("__data__") {
                            child.set_any("__data__", d);
                        }
                        SelectedNode::Node(child)
                    })
                } else {
                    None
                };
                subgroup.push(subnode);
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents.clone())
    }

    /// Consuming form of `select`.
    pub fn select_self(self, selector: &str) -> Self {
        self.select(selector)
    }

    /// The JavaScript's `selection.selectAll(selector)`.
    ///
    /// For each selected element, selects all descendants matching the selector.
    /// The resulting elements are grouped by their parent node in the original selection.
    pub fn select_all(&self, selector: &str) -> Self {
        let mut subgroups = Vec::new();
        let mut parents = Vec::new();
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let children = node.query_selector_all(selector);
                    let subgroup = children
                        .into_iter()
                        .map(|c| Some(SelectedNode::Node(c)))
                        .collect();
                    subgroups.push(subgroup);
                    parents.push(Some(node.clone()));
                }
            }
        }
        Selection::new(subgroups, parents)
    }

    /// The JavaScript's `selection.filter(selector)`.
    pub fn filter(&self, selector: &str) -> Self {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::new();
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if node.matches(selector) {
                        subgroup.push(Some(SelectedNode::Node(node.clone())));
                    }
                }
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents.clone())
    }

    /// The JavaScript's `selection.filter(function)`.
    pub fn filter_fn<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&N, Option<&dyn Any>, usize) -> bool,
    {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::new();
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    if predicate(node, d.as_deref().map(|r| r as &dyn Any), i) {
                        subgroup.push(Some(SelectedNode::Node(node.clone())));
                    }
                }
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents.clone())
    }

    // -----------------------------------------------------------------------
    // Data join: data, enter, exit, join
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.data(values)` (index-based join).
    pub fn data<D: Clone + 'static>(&mut self, data: &[D]) -> &mut Self {
        let mut update_groups = Vec::with_capacity(self._groups.len());
        let mut enter_groups = Vec::with_capacity(self._groups.len());
        let mut exit_groups = Vec::with_capacity(self._groups.len());

        for (j, group) in self._groups.iter().enumerate() {
            let parent = self._parents.get(j).cloned().flatten();
            let group_len = group.len();
            let data_len = data.len();

            let mut enter_group = Vec::with_capacity(data_len);
            let mut update_group = Vec::with_capacity(data_len);
            let mut exit_group = Vec::with_capacity(group_len);

            for i in 0..data_len {
                if i < group_len && group[i].is_some() {
                    let node_entry = group[i].as_ref().unwrap();
                    if let SelectedNode::Node(node) = node_entry {
                        node.set_data(data[i].clone());
                        update_group.push(Some(SelectedNode::Node(node.clone())));
                    } else {
                        enter_group.push(Some(EnterNode {
                            parent: parent.clone(),
                            next: None,
                            datum: Some(Rc::new(data[i].clone())),
                        }));
                        update_group.push(None);
                    }
                } else {
                    enter_group.push(Some(EnterNode {
                        parent: parent.clone(),
                        next: None,
                        datum: Some(Rc::new(data[i].clone())),
                    }));
                    update_group.push(None);
                }
            }

            for node_entry in group.iter().skip(data_len).flatten() {
                if let SelectedNode::Node(node) = node_entry {
                    exit_group.push(Some(node.clone()));
                }
            }

            // Connect enter nodes to following update node
            for i0 in 0..data_len {
                if let Some(enter_node) = enter_group.get_mut(i0).and_then(|opt| opt.as_mut()) {
                    let mut i1 = i0 + 1;
                    let mut next_node = None;
                    while i1 < data_len {
                        if let Some(Some(SelectedNode::Node(node))) = update_group.get(i1) {
                            next_node = Some(node.clone());
                            break;
                        }
                        i1 += 1;
                    }
                    enter_node.next = next_node;
                }
            }

            update_groups.push(update_group);
            enter_groups.push(enter_group);
            exit_groups.push(exit_group);
        }

        self._groups = update_groups;
        self._enter = Some(enter_groups);
        self._exit = Some(exit_groups);
        self
    }

    /// The JavaScript's `selection.data(values, key)` (key-based join).
    pub fn data_key<D: Clone + 'static, K>(&mut self, data: &[D], key: K) -> &mut Self
    where
        K: Fn(&Option<N>, Option<&D>, usize) -> String,
    {
        let mut update_groups = Vec::with_capacity(self._groups.len());
        let mut enter_groups = Vec::with_capacity(self._groups.len());
        let mut exit_groups = Vec::with_capacity(self._groups.len());

        for (j, group) in self._groups.iter().enumerate() {
            let parent = self._parents.get(j).cloned().flatten();
            let mut node_by_key = HashMap::new();
            let mut key_values = Vec::with_capacity(group.len());
            let mut exit_group = Vec::new();

            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_data::<D>();
                    let k = key(&Some(node.clone()), d.as_ref(), i);
                    key_values.push(k.clone());
                    if let std::collections::hash_map::Entry::Vacant(e) = node_by_key.entry(k) {
                        e.insert(node.clone());
                    } else {
                        exit_group.push(Some(node.clone()));
                    }
                } else {
                    key_values.push(String::new());
                }
            }

            let mut enter_group = Vec::with_capacity(data.len());
            let mut update_group = Vec::with_capacity(data.len());

            for (i, datum) in data.iter().enumerate() {
                let k = key(&parent, Some(datum), i);
                if let Some(node) = node_by_key.remove(&k) {
                    node.set_data(datum.clone());
                    update_group.push(Some(SelectedNode::Node(node)));
                    enter_group.push(None);
                } else {
                    enter_group.push(Some(EnterNode {
                        parent: parent.clone(),
                        next: None,
                        datum: Some(Rc::new(datum.clone())),
                    }));
                    update_group.push(None);
                }
            }

            // Any remaining nodes in map go to exit
            for node in node_by_key.into_values() {
                exit_group.push(Some(node));
            }

            // Connect enter nodes to following update node
            for i0 in 0..data.len() {
                if let Some(enter_node) = enter_group.get_mut(i0).and_then(|opt| opt.as_mut()) {
                    let mut i1 = i0 + 1;
                    let mut next_node = None;
                    while i1 < data.len() {
                        if let Some(Some(SelectedNode::Node(node))) = update_group.get(i1) {
                            next_node = Some(node.clone());
                            break;
                        }
                        i1 += 1;
                    }
                    enter_node.next = next_node;
                }
            }

            update_groups.push(update_group);
            enter_groups.push(enter_group);
            exit_groups.push(exit_group);
        }

        self._groups = update_groups;
        self._enter = Some(enter_groups);
        self._exit = Some(exit_groups);
        self
    }

    /// The JavaScript's `selection.enter()`.
    pub fn enter(&self) -> Self {
        let enter_groups = match &self._enter {
            Some(egs) => egs
                .iter()
                .map(|g| {
                    g.iter()
                        .map(|e_opt| e_opt.as_ref().map(|e| SelectedNode::Enter(e.clone())))
                        .collect()
                })
                .collect(),
            None => self._groups.iter().map(|g| vec![None; g.len()]).collect(),
        };
        Selection::new(enter_groups, self._parents.clone())
    }

    /// The JavaScript's `selection.exit()`.
    pub fn exit(&self) -> Self {
        let exit_groups = match &self._exit {
            Some(egs) => egs
                .iter()
                .map(|g| {
                    g.iter()
                        .map(|n_opt| n_opt.as_ref().map(|n| SelectedNode::Node(n.clone())))
                        .collect()
                })
                .collect(),
            None => self._groups.iter().map(|g| vec![None; g.len()]).collect(),
        };
        Selection::new(exit_groups, self._parents.clone())
    }

    /// The JavaScript's `selection.join(enter [, update] [, exit])`.
    pub fn join_tag(&mut self, tag: &str) -> Self {
        let enter = self.enter().append(tag);
        let exit = self.exit();
        exit.remove();
        enter.merge(self).order()
    }

    /// The JavaScript's `selection.merge(other)`.
    pub fn merge(&self, other: &Self) -> Self {
        let m0 = self._groups.len();
        let m1 = other._groups.len();
        let m = m0.min(m1);
        let mut merges = Vec::with_capacity(m0);

        for j in 0..m {
            let g0 = &self._groups[j];
            let g1 = &other._groups[j];
            let n = g0.len();
            let mut merge_group = Vec::with_capacity(n);
            for i in 0..n {
                let node = g0
                    .get(i)
                    .cloned()
                    .flatten()
                    .or_else(|| g1.get(i).cloned().flatten());
                merge_group.push(node);
            }
            merges.push(merge_group);
        }
        for j in m..m0 {
            merges.push(self._groups[j].clone());
        }

        Selection::new(merges, self._parents.clone())
    }

    /// The JavaScript's `selection.order()`.
    pub fn order(self) -> Self {
        for group in &self._groups {
            if group.is_empty() {
                continue;
            }
            let mut next: Option<N> = None;
            for node_opt in group.iter().rev() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if let Some(nxt) = &next {
                        if (node.compare_document_position(nxt) & 4) == 0 {
                            if let Some(parent) = nxt.parent_node() {
                                parent.insert_before(node.clone(), Some(nxt));
                            }
                        }
                    }
                    next = Some(node.clone());
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.sort(compare)`.
    pub fn sort<F>(mut self, compare: F) -> Self
    where
        F: Fn(&Option<Rc<dyn Any>>, &Option<Rc<dyn Any>>) -> Ordering,
    {
        for group in &mut self._groups {
            group.sort_by(|a, b| {
                let da = a.as_ref().and_then(|n| n.datum());
                let db = b.as_ref().and_then(|n| n.datum());
                compare(&da, &db)
            });
        }
        self.order()
    }

    // -----------------------------------------------------------------------
    // Execution and iteration: call, each, nodes, node, size, empty
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.call(function, ...args)`, consuming form.
    pub fn call<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
        f(self)
    }

    /// The JavaScript's `selection.call(function, ...args)`, mutating form.
    pub fn call_ref<F: FnOnce(&mut Self)>(&mut self, f: F) -> &mut Self {
        f(self);
        self
    }

    /// The JavaScript's `selection.nodes()`.
    pub fn nodes(&self) -> Vec<N> {
        let mut list = Vec::new();
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(n)) = node_opt {
                    list.push(n.clone());
                }
            }
        }
        list
    }

    /// The JavaScript's `selection.node()`.
    pub fn node(&self) -> Option<N> {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(n)) = node_opt {
                    return Some(n.clone());
                }
            }
        }
        None
    }

    /// The JavaScript's `selection.size()`.
    pub fn size(&self) -> usize {
        let mut count = 0;
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(_)) = node_opt {
                    count += 1;
                }
            }
        }
        count
    }

    /// The JavaScript's `selection.empty()`.
    pub fn empty(&self) -> bool {
        self.node().is_none()
    }

    /// Alias for `empty`.
    pub fn is_empty(&self) -> bool {
        self.empty()
    }

    /// The JavaScript's `selection.each(function)`.
    pub fn each<F>(&self, mut callback: F) -> &Self
    where
        F: FnMut(&N, Option<&dyn Any>, usize),
    {
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(n)) = node_opt {
                    let d = n.get_any("__data__");
                    callback(n, d.as_deref().map(|r| r as &dyn Any), i);
                }
            }
        }
        self
    }

    // -----------------------------------------------------------------------
    // Attributes, styles, properties, classes, text, and html
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.attr(name, value)`, consuming form.
    pub fn attr<V: IntoAttrValue<N>>(mut self, name: &str, value: V) -> Self {
        self.set_attr(name, value);
        self
    }

    /// Set an attribute using a typed datum accessor `fn(&D, usize) -> value`.
    pub fn attr_data<D: Clone + 'static, R, F>(self, name: &str, mut f: F) -> Self
    where
        F: FnMut(&D, usize) -> R,
        R: IntoOptionString,
    {
        let ns = namespace(name);
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if let Some(d) = node.get_data::<D>() {
                        let v = f(&d, i).into_option_string();
                        match v {
                            Some(val) => {
                                if let Some(ref space) = ns.space {
                                    node.set_attribute_ns(Some(space), &ns.local, &val);
                                } else {
                                    node.set_attribute(&ns.local, &val);
                                }
                            }
                            None => {
                                if let Some(ref space) = ns.space {
                                    node.remove_attribute_ns(Some(space), &ns.local);
                                } else {
                                    node.remove_attribute(&ns.local);
                                }
                            }
                        }
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.attr(name, value)`, mutating form.
    pub fn set_attr<V: IntoAttrValue<N>>(&mut self, name: &str, value: V) -> &mut Self {
        let ns = namespace(name);
        let mut val_fn = value.into_attr_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let v = val_fn(node.clone(), d, i);
                    match v {
                        Some(val) => {
                            if let Some(ref space) = ns.space {
                                node.set_attribute_ns(Some(space), &ns.local, &val);
                            } else {
                                node.set_attribute(&ns.local, &val);
                            }
                        }
                        None => {
                            if let Some(ref space) = ns.space {
                                node.remove_attribute_ns(Some(space), &ns.local);
                            } else {
                                node.remove_attribute(&ns.local);
                            }
                        }
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.attr(name)` getter.
    pub fn get_attr(&self, name: &str) -> Option<String> {
        let node = self.node()?;
        let ns = namespace(name);
        if let Some(ref space) = ns.space {
            node.get_attribute_ns(Some(space), &ns.local)
        } else {
            node.get_attribute(&ns.local)
        }
    }

    /// The JavaScript's `selection.style(name, value)`, consuming form.
    pub fn style<V: IntoStyleValue<N>>(mut self, name: &str, value: V) -> Self {
        self.set_style(name, value);
        self
    }

    /// The JavaScript's `selection.style(name, value)`, mutating form.
    pub fn set_style<V: IntoStyleValue<N>>(&mut self, name: &str, value: V) -> &mut Self {
        self.set_style_priority(name, value, "")
    }

    /// The JavaScript's `selection.style(name, value, priority)`.
    pub fn style_priority<V: IntoStyleValue<N>>(
        mut self,
        name: &str,
        value: V,
        priority: &str,
    ) -> Self {
        self.set_style_priority(name, value, priority);
        self
    }

    /// Mutating form of `style_priority`.
    pub fn set_style_priority<V: IntoStyleValue<N>>(
        &mut self,
        name: &str,
        value: V,
        priority: &str,
    ) -> &mut Self {
        let mut val_fn = value.into_style_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let v = val_fn(node.clone(), d, i);
                    match v {
                        Some(val) => node.set_style_property(name, &val, priority),
                        None => node.remove_style_property(name),
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.style(name)` getter.
    pub fn get_style(&self, name: &str) -> String {
        self.node()
            .map(|n| n.get_style_property(name))
            .unwrap_or_default()
    }

    /// The JavaScript's `selection.property(name, value)`, consuming form.
    pub fn property<V: IntoPropertyValue<N>>(mut self, name: &str, value: V) -> Self {
        self.set_property(name, value);
        self
    }

    /// The JavaScript's `selection.property(name, value)`, mutating form.
    pub fn set_property<V: IntoPropertyValue<N>>(&mut self, name: &str, value: V) -> &mut Self {
        let mut val_fn = value.into_property_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let v = val_fn(node.clone(), d, i);
                    match v {
                        Some(val) => node.set_property(name, &val),
                        None => node.remove_property(name),
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.property(name)` getter.
    pub fn get_property(&self, name: &str) -> Option<String> {
        self.node().and_then(|n| n.get_property(name))
    }

    /// The JavaScript's `selection.classed(names, value)`, consuming form.
    pub fn classed<V: IntoClassedValue<N>>(mut self, names: &str, value: V) -> Self {
        self.set_classed(names, value);
        self
    }

    /// The JavaScript's `selection.classed(names, value)`, mutating form.
    pub fn set_classed<V: IntoClassedValue<N>>(&mut self, names: &str, value: V) -> &mut Self {
        let class_list: Vec<&str> = names.split_whitespace().collect();
        let mut val_fn = value.into_classed_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let add = val_fn(node.clone(), d, i);
                    for &cls in &class_list {
                        if add {
                            node.class_list_add(cls);
                        } else {
                            node.class_list_remove(cls);
                        }
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.classed(names)` getter.
    pub fn get_classed(&self, names: &str) -> bool {
        let Some(node) = self.node() else {
            return false;
        };
        names
            .split_whitespace()
            .all(|cls| node.class_list_contains(cls))
    }

    /// The JavaScript's `selection.text(value)`, consuming form.
    pub fn text<V: IntoTextValue<N>>(mut self, value: V) -> Self {
        self.set_text(value);
        self
    }

    /// The JavaScript's `selection.text(value)`, mutating form.
    pub fn set_text<V: IntoTextValue<N>>(&mut self, value: V) -> &mut Self {
        let mut val_fn = value.into_text_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let txt = val_fn(node.clone(), d, i);
                    node.set_text_content(&txt);
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.text()` getter.
    pub fn get_text(&self) -> String {
        self.node().map(|n| n.text_content()).unwrap_or_default()
    }

    /// The JavaScript's `selection.html(value)`, consuming form.
    pub fn html<V: IntoHtmlValue<N>>(mut self, value: V) -> Self {
        self.set_html(value);
        self
    }

    /// The JavaScript's `selection.html(value)`, mutating form.
    pub fn set_html<V: IntoHtmlValue<N>>(&mut self, value: V) -> &mut Self {
        let mut val_fn = value.into_html_value();
        for group in &self._groups {
            for (i, node_opt) in group.iter().enumerate() {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let d = node.get_any("__data__");
                    let h = val_fn(node.clone(), d, i);
                    node.set_inner_html(&h);
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.html()` getter.
    pub fn get_html(&self) -> String {
        self.node().map(|n| n.inner_html()).unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // DOM mutation: raise, lower, append, insert, remove, clone
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.raise()`.
    pub fn raise(self) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if node.next_sibling().is_some() {
                        if let Some(parent) = node.parent_node() {
                            parent.append_child(node.clone());
                        }
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.lower()`.
    pub fn lower(self) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if node.previous_sibling().is_some() {
                        if let Some(parent) = node.parent_node() {
                            let first = parent.first_child();
                            parent.insert_before(node.clone(), first.as_ref());
                        }
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.append(name)`.
    pub fn append(self, name: &str) -> Self {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::with_capacity(group.len());
            for node_opt in group {
                let subnode = match node_opt {
                    Some(SelectedNode::Node(node)) => {
                        let child = node.create_element(name);
                        if let Some(d) = node.get_any("__data__") {
                            child.set_any("__data__", d);
                        }
                        node.append_child(child.clone());
                        Some(SelectedNode::Node(child))
                    }
                    Some(SelectedNode::Enter(enter)) => {
                        if let Some(parent) = &enter.parent {
                            let child = parent.create_element(name);
                            if let Some(d) = &enter.datum {
                                child.set_any("__data__", d.clone());
                            }
                            parent.insert_before(child.clone(), enter.next.as_ref());
                            Some(SelectedNode::Node(child))
                        } else {
                            None
                        }
                    }
                    None => None,
                };
                subgroup.push(subnode);
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents)
    }

    /// The JavaScript's `selection.insert(name, before)`.
    pub fn insert(self, name: &str, before: Option<&str>) -> Self {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::with_capacity(group.len());
            for node_opt in group {
                let subnode = match node_opt {
                    Some(SelectedNode::Node(node)) => {
                        let child = node.create_element(name);
                        if let Some(d) = node.get_any("__data__") {
                            child.set_any("__data__", d);
                        }
                        let before_node = before.and_then(|sel| node.query_selector(sel));
                        node.insert_before(child.clone(), before_node.as_ref());
                        Some(SelectedNode::Node(child))
                    }
                    Some(SelectedNode::Enter(enter)) => {
                        if let Some(parent) = &enter.parent {
                            let child = parent.create_element(name);
                            if let Some(d) = &enter.datum {
                                child.set_any("__data__", d.clone());
                            }
                            let before_node = before
                                .and_then(|sel| parent.query_selector(sel))
                                .or_else(|| enter.next.clone());
                            parent.insert_before(child.clone(), before_node.as_ref());
                            Some(SelectedNode::Node(child))
                        } else {
                            None
                        }
                    }
                    None => None,
                };
                subgroup.push(subnode);
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents)
    }

    /// The JavaScript's `selection.remove()`.
    pub fn remove(self) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    if let Some(parent) = node.parent_node() {
                        parent.remove_child(node);
                    }
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.clone(deep)`.
    pub fn clone_selection(&self, deep: bool) -> Self {
        let mut subgroups = Vec::with_capacity(self._groups.len());
        for group in &self._groups {
            let mut subgroup = Vec::with_capacity(group.len());
            for node_opt in group {
                let subnode = if let Some(SelectedNode::Node(node)) = node_opt {
                    let clone = node.clone_node(deep);
                    if let Some(parent) = node.parent_node() {
                        let nxt = node.next_sibling();
                        parent.insert_before(clone.clone(), nxt.as_ref());
                    }
                    Some(SelectedNode::Node(clone))
                } else {
                    None
                };
                subgroup.push(subnode);
            }
            subgroups.push(subgroup);
        }
        Selection::new(subgroups, self._parents.clone())
    }

    /// The JavaScript's `selection.datum(value)` setter.
    pub fn datum<D: Clone + 'static>(self, value: D) -> Self {
        let rc = Rc::new(value);
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    node.set_any("__data__", rc.clone());
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.datum()` getter.
    pub fn get_datum<D: Clone + 'static>(&self) -> Option<D> {
        self.node().and_then(|n| n.get_data::<D>())
    }

    // -----------------------------------------------------------------------
    // Event listeners and dispatch
    // -----------------------------------------------------------------------

    /// The JavaScript's `selection.dispatch(type, params)`.
    pub fn dispatch(self, event_type: &str) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    node.dispatch_event(event_type);
                }
            }
        }
        self
    }

    /// The JavaScript's `selection.transition([name])`.
    pub fn transition(&self, name: Option<&str>) -> crate::transition::Transition<N> {
        crate::transition::transition_selection(self, name)
    }

    /// The JavaScript's `selection.interrupt([name])`.
    pub fn interrupt(&self, name: Option<&str>) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    crate::transition::interrupt(node, name);
                }
            }
        }
        self.clone()
    }
}

// ---------------------------------------------------------------------------
// Conversion traits for values passed to attr, style, text, html, etc.
// ---------------------------------------------------------------------------

/// A trait for types that can convert into an optional string value.
pub trait IntoOptionString {
    /// Convert to Option<String>.
    fn into_option_string(self) -> Option<String>;
}

impl IntoOptionString for Option<String> {
    fn into_option_string(self) -> Option<String> {
        self
    }
}

impl IntoOptionString for Option<&str> {
    fn into_option_string(self) -> Option<String> {
        self.map(|s| s.to_string())
    }
}

impl IntoOptionString for String {
    fn into_option_string(self) -> Option<String> {
        Some(self)
    }
}

impl IntoOptionString for &str {
    fn into_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOptionString for f64 {
    fn into_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOptionString for i32 {
    fn into_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOptionString for usize {
    fn into_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

type AttrFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> Option<String>>;

/// A value that can be set as an attribute value.
pub trait IntoAttrValue<N: Dom> {
    /// Convert this value into an attribute evaluator function.
    fn into_attr_value(self) -> AttrFn<N>;
}

impl<N: Dom> IntoAttrValue<N> for &str {
    fn into_attr_value(self) -> AttrFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoAttrValue<N> for String {
    fn into_attr_value(self) -> AttrFn<N> {
        let s = self;
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoAttrValue<N> for f64 {
    fn into_attr_value(self) -> AttrFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoAttrValue<N> for i32 {
    fn into_attr_value(self) -> AttrFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoAttrValue<N> for usize {
    fn into_attr_value(self) -> AttrFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom, F, R> IntoAttrValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> R + 'static,
    R: IntoOptionString,
{
    fn into_attr_value(self) -> AttrFn<N> {
        let mut f = self;
        Box::new(move |node, d, i| f(node, d, i).into_option_string())
    }
}

type StyleFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> Option<String>>;

/// A value that can be set as an inline style value.
pub trait IntoStyleValue<N: Dom> {
    /// Convert this value into a style evaluator function.
    fn into_style_value(self) -> StyleFn<N>;
}

impl<N: Dom> IntoStyleValue<N> for &str {
    fn into_style_value(self) -> StyleFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoStyleValue<N> for String {
    fn into_style_value(self) -> StyleFn<N> {
        let s = self;
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoStyleValue<N> for f64 {
    fn into_style_value(self) -> StyleFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom, F, R> IntoStyleValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> R + 'static,
    R: IntoOptionString,
{
    fn into_style_value(self) -> StyleFn<N> {
        let mut f = self;
        Box::new(move |node, d, i| f(node, d, i).into_option_string())
    }
}

type TextFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> String>;

/// A value that can be set as text content.
pub trait IntoTextValue<N: Dom> {
    /// Convert this value into a text evaluator function.
    fn into_text_value(self) -> TextFn<N>;
}

impl<N: Dom> IntoTextValue<N> for &str {
    fn into_text_value(self) -> TextFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom> IntoTextValue<N> for String {
    fn into_text_value(self) -> TextFn<N> {
        let s = self;
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom> IntoTextValue<N> for f64 {
    fn into_text_value(self) -> TextFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom> IntoTextValue<N> for i32 {
    fn into_text_value(self) -> TextFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom> IntoTextValue<N> for usize {
    fn into_text_value(self) -> TextFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom, F, R> IntoTextValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> R + 'static,
    R: ToString,
{
    fn into_text_value(self) -> TextFn<N> {
        let mut f = self;
        Box::new(move |node, d, i| f(node, d, i).to_string())
    }
}

type HtmlFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> String>;

/// A value that can be set as inner HTML.
pub trait IntoHtmlValue<N: Dom> {
    /// Convert this value into an HTML evaluator function.
    fn into_html_value(self) -> HtmlFn<N>;
}

impl<N: Dom> IntoHtmlValue<N> for &str {
    fn into_html_value(self) -> HtmlFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom> IntoHtmlValue<N> for String {
    fn into_html_value(self) -> HtmlFn<N> {
        let s = self;
        Box::new(move |_, _, _| s.clone())
    }
}

impl<N: Dom, F, R> IntoHtmlValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> R + 'static,
    R: ToString,
{
    fn into_html_value(self) -> HtmlFn<N> {
        let mut f = self;
        Box::new(move |node, d, i| f(node, d, i).to_string())
    }
}

type ClassedFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> bool>;

/// A value that can be set as a class toggle.
pub trait IntoClassedValue<N: Dom> {
    /// Convert this value into a class toggle evaluator function.
    fn into_classed_value(self) -> ClassedFn<N>;
}

impl<N: Dom> IntoClassedValue<N> for bool {
    fn into_classed_value(self) -> ClassedFn<N> {
        let b = self;
        Box::new(move |_, _, _| b)
    }
}

impl<N: Dom, F> IntoClassedValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> bool + 'static,
{
    fn into_classed_value(self) -> ClassedFn<N> {
        Box::new(self)
    }
}

type PropertyFn<N> = Box<dyn FnMut(N, Option<Rc<dyn Any>>, usize) -> Option<String>>;

/// A value that can be set as a DOM property.
pub trait IntoPropertyValue<N: Dom> {
    /// Convert this value into a property evaluator function.
    fn into_property_value(self) -> PropertyFn<N>;
}

impl<N: Dom> IntoPropertyValue<N> for &str {
    fn into_property_value(self) -> PropertyFn<N> {
        let s = self.to_string();
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom> IntoPropertyValue<N> for String {
    fn into_property_value(self) -> PropertyFn<N> {
        let s = self;
        Box::new(move |_, _, _| Some(s.clone()))
    }
}

impl<N: Dom, F, R> IntoPropertyValue<N> for F
where
    F: FnMut(N, Option<Rc<dyn Any>>, usize) -> R + 'static,
    R: IntoOptionString,
{
    fn into_property_value(self) -> PropertyFn<N> {
        let mut f = self;
        Box::new(move |node, d, i| f(node, d, i).into_option_string())
    }
}

// ---------------------------------------------------------------------------
// d3.local
// ---------------------------------------------------------------------------

static NEXT_LOCAL_ID: AtomicUsize = AtomicUsize::new(1);

/// The JavaScript's `d3.local()`.
///
/// Declares a local variable with scoping over the DOM tree. Reading a local
/// variable walks up the ancestor tree until an assigned value is found.
pub struct Local<T> {
    id: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Clone + 'static> Default for Local<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + 'static> Local<T> {
    /// Create a new local variable.
    pub fn new() -> Self {
        let num = NEXT_LOCAL_ID.fetch_add(1, AtomicOrdering::Relaxed);
        Local {
            id: format!("@{num:x}"),
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the value of this local variable on the given DOM node.
    pub fn set<N: Dom>(&self, node: &N, value: T) {
        node.set_any(&self.id, Rc::new(value));
    }

    /// Read the value of this local variable from the node, or walk up to ancestors.
    pub fn get<N: Dom>(&self, node: &N) -> Option<T> {
        let mut curr = Some(node.clone());
        while let Some(n) = curr {
            if let Some(val) = n
                .get_any(&self.id)
                .and_then(|a| a.downcast_ref::<T>().cloned())
            {
                return Some(val);
            }
            curr = n.parent_node();
        }
        None
    }

    /// Remove the value of this local variable from the given DOM node.
    pub fn remove<N: Dom>(&self, node: &N) {
        node.remove_any(&self.id);
    }
}

/// The JavaScript's `d3.local()` constructor.
pub fn local<T: Clone + 'static>() -> Local<T> {
    Local::new()
}

// ---------------------------------------------------------------------------
// Standalone functions: select, selectAll, create, mouse, touch, point
// ---------------------------------------------------------------------------

/// The JavaScript's `d3.select(selector)`.
///
/// Selects the first element matching the selector from the document element.
pub fn select(selector: &str) -> Selection<SimpleNode> {
    let doc = create_document();
    let root = doc.document_element().unwrap_or_else(|| doc.clone());
    let match_opt = root.query_selector(selector);
    let group = vec![match_opt.map(SelectedNode::Node)];
    Selection::new(vec![group], vec![Some(root)])
}

/// The JavaScript's `d3.select(node)` for an existing DOM node.
pub fn select_node<N: Dom>(node: N) -> Selection<N> {
    Selection::from_node(node)
}

/// The JavaScript's `d3.selectAll(selector)`.
///
/// Selects all elements matching the selector from the document element.
pub fn select_all(selector: &str) -> Selection<SimpleNode> {
    let doc = create_document();
    let root = doc.document_element().unwrap_or_else(|| doc.clone());
    let matches = root.query_selector_all(selector);
    let group = matches
        .into_iter()
        .map(|n| Some(SelectedNode::Node(n)))
        .collect();
    Selection::new(vec![group], vec![Some(root)])
}

/// The JavaScript's `d3.selectAll(nodes)` for a slice of DOM nodes.
pub fn select_all_nodes<N: Dom>(nodes: Vec<N>) -> Selection<N> {
    Selection::from_nodes(nodes)
}

/// The JavaScript's `d3.create(name)`.
///
/// Creates a new element with the given tag name and returns a selection wrapping it.
pub fn create(name: &str) -> Selection<SimpleNode> {
    let el = create_element(name);
    Selection::from_node(el)
}

/// The JavaScript's `clientPoint(node, event)`.
pub fn client_point<N: Dom>(node: &N, client_x: f64, client_y: f64) -> [f64; 2] {
    if let Some(ctm) = node.get_screen_ctm() {
        // [a, b, c, d, e, f] 2D affine matrix inverse transform
        let det = ctm[0] * ctm[3] - ctm[1] * ctm[2];
        if det != 0.0 {
            let inv_a = ctm[3] / det;
            let inv_b = -ctm[1] / det;
            let inv_c = -ctm[2] / det;
            let inv_d = ctm[0] / det;
            let inv_e = (ctm[2] * ctm[5] - ctm[3] * ctm[4]) / det;
            let inv_f = (ctm[1] * ctm[4] - ctm[0] * ctm[5]) / det;
            let x = client_x * inv_a + client_y * inv_c + inv_e;
            let y = client_x * inv_b + client_y * inv_d + inv_f;
            return [x, y];
        }
    }
    let rect = node.get_bounding_client_rect();
    [
        client_x - rect.0 - node.client_left(),
        client_y - rect.1 - node.client_top(),
    ]
}

/// The JavaScript's `d3.mouse(node)`.
///
/// Returns the x and y coordinates of the current mouse event relative to the specified node.
pub fn mouse<N: Dom>(node: &N) -> [f64; 2] {
    let ev = source_event().unwrap_or_default();
    client_point(node, ev.client_x, ev.client_y)
}

/// The JavaScript's `d3.touch(node, identifier)`.
pub fn touch<N: Dom>(node: &N, identifier: i32) -> Option<[f64; 2]> {
    let ev = source_event().unwrap_or_default();
    for t in &ev.touches {
        if t.identifier == identifier {
            return Some(client_point(node, t.client_x, t.client_y));
        }
    }
    for t in &ev.changed_touches {
        if t.identifier == identifier {
            return Some(client_point(node, t.client_x, t.client_y));
        }
    }
    None
}

/// The JavaScript's `d3.touches(node)`.
pub fn touches<N: Dom>(node: &N) -> Vec<[f64; 2]> {
    let ev = source_event().unwrap_or_default();
    ev.touches
        .iter()
        .map(|t| client_point(node, t.client_x, t.client_y))
        .collect()
}

/// The JavaScript's `style(node, name)`.
pub fn style<N: Dom>(node: &N, name: &str) -> String {
    node.get_style_property(name)
}

/// The JavaScript's `window(node)`.
pub fn window<N: Dom>(node: &N) -> Option<N> {
    node.default_view()
}
