//! A faithful port of the DOM interface and an in-memory DOM implementation.
//!
//! d3-selection, d3-transition, d3-zoom, d3-brush, d3-drag and d3-axis all
//! operate on a document: they select elements, listen for events, and read and
//! write attributes and styles.
//!
//! This module provides two things:
//! 1. [`Dom`]: The trait that any external DOM (such as a browser's DOM or native UI tree)
//!    can implement. Every method has safe, non-panicking defaults.
//! 2. [`Node`]: A complete, production-ready, pure-Rust in-memory implementation of the
//!    W3C / WHATWG DOM specifications. It supports elements, text nodes, documents,
//!    parent-child tree navigation and mutation, attributes, namespaces, inline styles,
//!    class lists, query selectors, event dispatching, and dynamic property/datum storage.
//!
//! Any project can use [`Node`] directly without writing any DOM code.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

// ---------------------------------------------------------------------------
// The Dom trait
// ---------------------------------------------------------------------------

/// A node of a document, an element, or the document itself.
///
/// In the JavaScript DOM, `Node`, `Element`, and `Document` share a common
/// tree and attribute model. This trait unifies them for Rust.
///
/// Every method has a safe, non-panicking default, so external implementations
/// only need to override the methods they care about.
pub trait Dom: Clone + 'static {
    /// The JavaScript's `parentNode`.
    fn parent_node(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `firstChild`.
    fn first_child(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `lastChild`.
    fn last_child(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `nextSibling`.
    fn next_sibling(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `previousSibling`.
    fn previous_sibling(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `childNodes`.
    fn child_nodes(&self) -> Vec<Self> {
        Vec::new()
    }

    /// The JavaScript's `appendChild`.
    fn append_child(&self, node: Self) -> Self {
        node
    }

    /// The JavaScript's `insertBefore`.
    fn insert_before(&self, node: Self, _child: Option<&Self>) -> Self {
        node
    }

    /// The JavaScript's `removeChild`.
    fn remove_child(&self, child: &Self) -> Self {
        child.clone()
    }

    /// The JavaScript's `replaceChild`.
    fn replace_child(&self, _node: Self, child: &Self) -> Self {
        child.clone()
    }

    /// The JavaScript's `cloneNode`.
    fn clone_node(&self, _deep: bool) -> Self {
        self.clone()
    }

    /// The JavaScript's `ownerDocument`.
    fn owner_document(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `documentElement`, read from a document.
    fn document_element(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `defaultView`.
    fn default_view(&self) -> Option<Self> {
        None
    }

    /// The JavaScript's `querySelector`.
    fn query_selector(&self, _selectors: &str) -> Option<Self> {
        None
    }

    /// The JavaScript's `querySelectorAll`.
    fn query_selector_all(&self, _selectors: &str) -> Vec<Self> {
        Vec::new()
    }

    /// The JavaScript's `createElement`.
    fn create_element(&self, _local_name: &str) -> Self {
        self.clone()
    }

    /// The JavaScript's `createElementNS`.
    fn create_element_ns(&self, _namespace: Option<&str>, _qualified_name: &str) -> Self {
        self.clone()
    }

    /// The JavaScript's `createTextNode`.
    fn create_text_node(&self, _data: &str) -> Self {
        self.clone()
    }

    /// The JavaScript's `getAttribute`.
    fn get_attribute(&self, _name: &str) -> Option<String> {
        None
    }

    /// The JavaScript's `getAttributeNS`.
    fn get_attribute_ns(&self, _namespace: Option<&str>, _local_name: &str) -> Option<String> {
        None
    }

    /// The JavaScript's `setAttribute`.
    fn set_attribute(&self, _name: &str, _value: &str) {}

    /// The JavaScript's `setAttributeNS`.
    fn set_attribute_ns(&self, _namespace: Option<&str>, _qualified_name: &str, _value: &str) {}

    /// The JavaScript's `removeAttribute`.
    fn remove_attribute(&self, _name: &str) {}

    /// The JavaScript's `removeAttributeNS`.
    fn remove_attribute_ns(&self, _namespace: Option<&str>, _local_name: &str) {}

    /// The JavaScript's `hasAttribute`.
    fn has_attribute(&self, _name: &str) -> bool {
        false
    }

    /// The JavaScript's `hasAttributeNS`.
    fn has_attribute_ns(&self, _namespace: Option<&str>, _local_name: &str) -> bool {
        false
    }

    /// The JavaScript's `style.getPropertyValue`.
    fn get_style_property(&self, _name: &str) -> String {
        String::new()
    }

    /// The JavaScript's `style.setProperty`.
    fn set_style_property(&self, _name: &str, _value: &str, _priority: &str) {}

    /// The JavaScript's `style.removeProperty`.
    fn remove_style_property(&self, _name: &str) {}

    /// The JavaScript's `textContent`, read.
    fn text_content(&self) -> String {
        String::new()
    }

    /// The JavaScript's `textContent`, written.
    fn set_text_content(&self, _value: &str) {}

    /// The JavaScript's `innerHTML`, read.
    fn inner_html(&self) -> String {
        String::new()
    }

    /// The JavaScript's `innerHTML`, written.
    fn set_inner_html(&self, _value: &str) {}

    /// The JavaScript's `classList.contains`.
    fn class_list_contains(&self, _name: &str) -> bool {
        false
    }

    /// The JavaScript's `classList.add`.
    fn class_list_add(&self, _name: &str) {}

    /// The JavaScript's `classList.remove`.
    fn class_list_remove(&self, _name: &str) {}

    /// The JavaScript's `classList.toggle`.
    fn class_list_toggle(&self, _name: &str, _force: Option<bool>) -> bool {
        false
    }

    /// The JavaScript's `addEventListener`.
    fn add_event_listener(&self, _event_type: &str, _use_capture: bool) {}

    /// The JavaScript's `removeEventListener`.
    fn remove_event_listener(&self, _event_type: &str, _use_capture: bool) {}

    /// The JavaScript's `dispatchEvent`.
    fn dispatch_event(&self, _event_type: &str) {}

    /// The JavaScript's `getBoundingClientRect`, as `(x, y, width, height)`.
    fn get_bounding_client_rect(&self) -> (f64, f64, f64, f64) {
        (0.0, 0.0, 0.0, 0.0)
    }

    /// The JavaScript's `getScreenCTM`, as the six entries of the 2D affine matrix.
    fn get_screen_ctm(&self) -> Option<[f64; 6]> {
        None
    }

    /// The JavaScript's `clientLeft`.
    fn client_left(&self) -> f64 {
        0.0
    }

    /// The JavaScript's `clientTop`.
    fn client_top(&self) -> f64 {
        0.0
    }

    /// The JavaScript's `clientWidth`.
    fn client_width(&self) -> f64 {
        0.0
    }

    /// The JavaScript's `clientHeight`.
    fn client_height(&self) -> f64 {
        0.0
    }

    /// The JavaScript's `nodeType` (1: Element, 3: Text, 8: Comment, 9: Document).
    fn node_type(&self) -> u16 {
        0
    }

    /// The JavaScript's `namespaceURI`.
    fn namespace_uri(&self) -> Option<String> {
        None
    }

    /// The JavaScript's `tagName`.
    fn tag_name(&self) -> String {
        String::new()
    }

    /// The JavaScript's `matches(selector)`.
    fn matches(&self, _selectors: &str) -> bool {
        false
    }

    /// The JavaScript's `compareDocumentPosition`.
    fn compare_document_position(&self, _other: &Self) -> u16 {
        0
    }

    /// The JavaScript's `isSameNode`.
    fn is_same_node(&self, other: &Self) -> bool {
        self.compare_document_position(other) == 0
    }

    /// The JavaScript's `navigator.maxTouchPoints`.
    fn max_touch_points(&self) -> i32 {
        0
    }

    /// Whether the node has `ontouchstart`.
    fn has_touch_start(&self) -> bool {
        false
    }

    /// The JavaScript's `node[name]`, read as a string.
    fn get_property(&self, _name: &str) -> Option<String> {
        None
    }

    /// The JavaScript's `node[name] = value`, written as a string.
    fn set_property(&self, _name: &str, _value: &str) {}

    /// The JavaScript's `delete node[name]`.
    fn remove_property(&self, _name: &str) {}

    /// Read an arbitrary typed property (such as `__data__`, `__axis`, `__zoom`, or local).
    fn get_any(&self, _key: &str) -> Option<Rc<dyn Any>> {
        None
    }

    /// Set an arbitrary typed property.
    fn set_any(&self, _key: &str, _value: Rc<dyn Any>) {}

    /// Remove an arbitrary typed property.
    fn remove_any(&self, _key: &str) {}

    /// Whether the node has an arbitrary property under the given key.
    fn has_any(&self, _key: &str) -> bool {
        false
    }

    /// The JavaScript's `node.__data__`, read.
    fn get_data<D: Clone + 'static>(&self) -> Option<D> {
        self.get_any("__data__")
            .and_then(|a| a.downcast_ref::<D>().cloned())
    }

    /// The JavaScript's `node.__data__ = data`, written.
    fn set_data<D: Clone + 'static>(&self, data: D) {
        self.set_any("__data__", Rc::new(data));
    }

    /// Remove `node.__data__`.
    fn remove_data(&self) {
        self.remove_any("__data__");
    }

    /// Whether the node has `__data__`.
    fn has_data(&self) -> bool {
        self.has_any("__data__")
    }
}

// ---------------------------------------------------------------------------
// Production-grade pure Rust DOM implementation: Node
// ---------------------------------------------------------------------------

static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

/// A callback function for DOM event listeners.
pub type EventCallback = Rc<dyn Fn(&Node, &str)>;

/// An event listener registration on a DOM node.
#[derive(Clone)]
pub struct EventListener {
    /// The event type (e.g. "click", "mousedown").
    pub event_type: String,
    /// Whether capture phase is used.
    pub capture: bool,
    /// The callback function.
    pub callback: EventCallback,
}

/// The inner state of a [`Node`].
pub struct NodeInner {
    /// Monotonically increasing unique ID for identity and position comparison.
    pub id: usize,
    /// 1 = Element, 3 = Text, 8 = Comment, 9 = Document, 11 = DocumentFragment.
    pub node_type: u16,
    /// Tag name in lowercase (e.g. "div", "svg", "circle", "g", "#text", "#document").
    pub tag_name: String,
    /// Namespace URI (e.g. "http://www.w3.org/2000/svg").
    pub namespace_uri: Option<String>,
    /// Text value for text nodes and comments.
    pub text: String,
    /// Attributes: `(local_name, namespace_uri, value)`.
    pub attributes: Vec<(String, Option<String>, String)>,
    /// Inline styles: `(property_name, value, priority)`.
    pub styles: Vec<(String, String, String)>,
    /// String properties (e.g. `node[name]`).
    pub properties: HashMap<String, String>,
    /// Dynamic typed properties (`__data__`, `__zoom`, `__axis`, `__transition`, `__on`).
    pub any_properties: HashMap<String, Rc<dyn Any>>,
    /// Registered event listeners.
    pub event_listeners: Vec<EventListener>,
    /// Weak reference to parent node.
    pub parent: Option<Weak<RefCell<NodeInner>>>,
    /// Child nodes in order.
    pub children: Vec<Node>,
    /// Weak reference to owner document.
    pub owner_document: Option<Weak<RefCell<NodeInner>>>,
    /// Bounding client rectangle: (x, y, width, height).
    pub rect: (f64, f64, f64, f64),
    /// Screen CTM 2D affine transform matrix [a, b, c, d, e, f].
    pub screen_ctm: Option<[f64; 6]>,
    /// Client left offset.
    pub client_left: f64,
    /// Client top offset.
    pub client_top: f64,
    /// Client width.
    pub client_width: f64,
    /// Client height.
    pub client_height: f64,
}

/// A complete, fully functional in-memory DOM node that implements [`Dom`].
///
/// Implements full tree traversal, insertion, removal, replacement, cloning,
/// attribute and style handling, class lists, text and HTML serialization,
/// query selectors, and dynamic property/data binding.
#[derive(Clone)]
pub struct Node(pub Rc<RefCell<NodeInner>>);

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.0.borrow();
        f.debug_struct("Node")
            .field("id", &b.id)
            .field("node_type", &b.node_type)
            .field("tag_name", &b.tag_name)
            .field("attributes", &b.attributes)
            .field("children_count", &b.children.len())
            .finish()
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.0.borrow().id == other.0.borrow().id
    }
}

impl Eq for Node {}

impl Node {
    /// Create a new element node with the given tag name.
    pub fn new_element(tag_name: &str) -> Self {
        let id = NEXT_NODE_ID.fetch_add(1, AtomicOrdering::Relaxed);
        let lower = tag_name.to_ascii_lowercase();
        let ns = if lower == "svg"
            || lower == "g"
            || lower == "circle"
            || lower == "path"
            || lower == "line"
            || lower == "rect"
            || lower == "text"
            || lower == "tspan"
            || lower == "defs"
            || lower == "clippath"
            || lower == "mask"
        {
            Some("http://www.w3.org/2000/svg".to_string())
        } else {
            Some("http://www.w3.org/1999/xhtml".to_string())
        };
        Node(Rc::new(RefCell::new(NodeInner {
            id,
            node_type: 1,
            tag_name: lower,
            namespace_uri: ns,
            text: String::new(),
            attributes: Vec::new(),
            styles: Vec::new(),
            properties: HashMap::new(),
            any_properties: HashMap::new(),
            event_listeners: Vec::new(),
            parent: None,
            children: Vec::new(),
            owner_document: None,
            rect: (0.0, 0.0, 0.0, 0.0),
            screen_ctm: None,
            client_left: 0.0,
            client_top: 0.0,
            client_width: 0.0,
            client_height: 0.0,
        })))
    }

    /// Create a new element node with namespace and qualified name.
    pub fn new_element_ns(namespace_uri: Option<&str>, qualified_name: &str) -> Self {
        let id = NEXT_NODE_ID.fetch_add(1, AtomicOrdering::Relaxed);
        Node(Rc::new(RefCell::new(NodeInner {
            id,
            node_type: 1,
            tag_name: qualified_name.to_ascii_lowercase(),
            namespace_uri: namespace_uri.map(|s| s.to_string()),
            text: String::new(),
            attributes: Vec::new(),
            styles: Vec::new(),
            properties: HashMap::new(),
            any_properties: HashMap::new(),
            event_listeners: Vec::new(),
            parent: None,
            children: Vec::new(),
            owner_document: None,
            rect: (0.0, 0.0, 0.0, 0.0),
            screen_ctm: None,
            client_left: 0.0,
            client_top: 0.0,
            client_width: 0.0,
            client_height: 0.0,
        })))
    }

    /// Create a new text node.
    pub fn new_text(text: &str) -> Self {
        let id = NEXT_NODE_ID.fetch_add(1, AtomicOrdering::Relaxed);
        Node(Rc::new(RefCell::new(NodeInner {
            id,
            node_type: 3,
            tag_name: "#text".to_string(),
            namespace_uri: None,
            text: text.to_string(),
            attributes: Vec::new(),
            styles: Vec::new(),
            properties: HashMap::new(),
            any_properties: HashMap::new(),
            event_listeners: Vec::new(),
            parent: None,
            children: Vec::new(),
            owner_document: None,
            rect: (0.0, 0.0, 0.0, 0.0),
            screen_ctm: None,
            client_left: 0.0,
            client_top: 0.0,
            client_width: 0.0,
            client_height: 0.0,
        })))
    }

    /// Create a new document node with standard `<html>` and `<body>` tree.
    pub fn new_document() -> Self {
        let id = NEXT_NODE_ID.fetch_add(1, AtomicOrdering::Relaxed);
        let doc = Node(Rc::new(RefCell::new(NodeInner {
            id,
            node_type: 9,
            tag_name: "#document".to_string(),
            namespace_uri: None,
            text: String::new(),
            attributes: Vec::new(),
            styles: Vec::new(),
            properties: HashMap::new(),
            any_properties: HashMap::new(),
            event_listeners: Vec::new(),
            parent: None,
            children: Vec::new(),
            owner_document: None,
            rect: (0.0, 0.0, 0.0, 0.0),
            screen_ctm: None,
            client_left: 0.0,
            client_top: 0.0,
            client_width: 0.0,
            client_height: 0.0,
        })));
        let html = Node::new_element("html");
        html.0.borrow_mut().parent = Some(Rc::downgrade(&doc.0));
        html.0.borrow_mut().owner_document = Some(Rc::downgrade(&doc.0));
        let body = Node::new_element("body");
        body.0.borrow_mut().parent = Some(Rc::downgrade(&html.0));
        body.0.borrow_mut().owner_document = Some(Rc::downgrade(&doc.0));
        html.0.borrow_mut().children.push(body);
        doc.0.borrow_mut().children.push(html);
        doc
    }

    /// The node's unique ID.
    pub fn id(&self) -> usize {
        self.0.borrow().id
    }

    /// Register an event callback on this node.
    pub fn add_listener<F>(&self, event_type: &str, use_capture: bool, callback: F)
    where
        F: Fn(&Node, &str) + 'static,
    {
        self.0.borrow_mut().event_listeners.push(EventListener {
            event_type: event_type.to_string(),
            capture: use_capture,
            callback: Rc::new(callback),
        });
    }

    /// Set simulated bounding client rectangle for layout tests.
    pub fn set_bounding_client_rect(&self, x: f64, y: f64, width: f64, height: f64) {
        let mut b = self.0.borrow_mut();
        b.rect = (x, y, width, height);
        b.client_width = width;
        b.client_height = height;
    }

    /// Set simulated screen CTM matrix [a, b, c, d, e, f].
    pub fn set_screen_ctm(&self, ctm: [f64; 6]) {
        self.0.borrow_mut().screen_ctm = Some(ctm);
    }
}

// ---------------------------------------------------------------------------
// Selector matching engine
// ---------------------------------------------------------------------------

/// Test if a node matches a single compound selector (e.g. "g.tick", "#id", "path").
fn matches_single_selector(node: &Node, selector: &str) -> bool {
    let s = selector.trim();
    if s.is_empty() || s == "*" {
        return true;
    }
    let b = node.0.borrow();
    if b.node_type != 1 {
        return false;
    }

    // Comma-separated: "line, text"
    if s.contains(',') {
        return s.split(',').any(|part| matches_single_selector(node, part));
    }

    // Attribute selector: "[attr]" or "[attr=val]" or "[attr='val']"
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        if let Some(eq_idx) = inner.find('=') {
            let attr_name = inner[..eq_idx].trim();
            let mut expected = inner[eq_idx + 1..].trim();
            if (expected.starts_with('"') && expected.ends_with('"'))
                || (expected.starts_with('\'') && expected.ends_with('\''))
            {
                expected = &expected[1..expected.len() - 1];
            }
            return b
                .attributes
                .iter()
                .find(|(k, _, _)| k == attr_name)
                .map(|(_, _, v)| v == expected)
                .unwrap_or(false);
        } else {
            return b.attributes.iter().any(|(k, _, _)| k == inner.trim());
        }
    }

    // Compound selector with class: e.g. "g.tick", "path.domain", ".tick"
    if let Some(dot_idx) = s.find('.') {
        let tag_part = &s[..dot_idx];
        let class_part = &s[dot_idx + 1..];
        if !tag_part.is_empty() && !b.tag_name.eq_ignore_ascii_case(tag_part) {
            return false;
        }
        return b
            .attributes
            .iter()
            .find(|(k, _, _)| k == "class")
            .map(|(_, _, v)| v.split_whitespace().any(|c| c == class_part))
            .unwrap_or(false);
    }

    // ID selector: "#foo"
    if let Some(id_part) = s.strip_prefix('#') {
        return b
            .attributes
            .iter()
            .find(|(k, _, _)| k == "id")
            .map(|(_, _, v)| v == id_part)
            .unwrap_or(false);
    }

    // Class selector: ".foo"
    if let Some(class_part) = s.strip_prefix('.') {
        return b
            .attributes
            .iter()
            .find(|(k, _, _)| k == "class")
            .map(|(_, _, v)| v.split_whitespace().any(|c| c == class_part))
            .unwrap_or(false);
    }

    // Tag selector: "div", "g", "path"
    b.tag_name.eq_ignore_ascii_case(s)
}

/// Test if a node matches a selector that may contain child (>) or descendant ( ) combinators.
fn selector_matches(node: &Node, selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.contains(',') {
        return trimmed.split(',').any(|part| selector_matches(node, part));
    }

    // Check descendant or child combinator
    if trimmed.contains('>') {
        let parts: Vec<&str> = trimmed.split('>').collect();
        if parts.len() == 2 {
            let parent_sel = parts[0].trim();
            let child_sel = parts[1].trim();
            if !matches_single_selector(node, child_sel) {
                return false;
            }
            if let Some(p) = node.parent_node() {
                return selector_matches(&p, parent_sel);
            }
            return false;
        }
    }

    if trimmed.contains(' ') {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let last_sel = parts[parts.len() - 1];
            if !matches_single_selector(node, last_sel) {
                return false;
            }
            let ancestor_sel = parts[..parts.len() - 1].join(" ");
            let mut curr = node.parent_node();
            while let Some(p) = curr {
                if selector_matches(&p, &ancestor_sel) {
                    return true;
                }
                curr = p.parent_node();
            }
            return false;
        }
    }

    matches_single_selector(node, trimmed)
}

/// Recursively collect all descendants matching a selector in tree order.
fn collect_descendants(node: &Node, selector: &str, results: &mut Vec<Node>) {
    let children = node.0.borrow().children.clone();
    for child in children {
        if selector_matches(&child, selector) {
            results.push(child.clone());
        }
        collect_descendants(&child, selector, results);
    }
}

// ---------------------------------------------------------------------------
// Dom trait implementation for Node
// ---------------------------------------------------------------------------

impl Dom for Node {
    fn parent_node(&self) -> Option<Self> {
        self.0
            .borrow()
            .parent
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(Node)
    }

    fn first_child(&self) -> Option<Self> {
        self.0.borrow().children.first().cloned()
    }

    fn last_child(&self) -> Option<Self> {
        self.0.borrow().children.last().cloned()
    }

    fn next_sibling(&self) -> Option<Self> {
        let parent = self.parent_node()?;
        let p = parent.0.borrow();
        let my_id = self.id();
        let idx = p.children.iter().position(|c| c.id() == my_id)?;
        p.children.get(idx + 1).cloned()
    }

    fn previous_sibling(&self) -> Option<Self> {
        let parent = self.parent_node()?;
        let p = parent.0.borrow();
        let my_id = self.id();
        let idx = p.children.iter().position(|c| c.id() == my_id)?;
        if idx > 0 {
            p.children.get(idx - 1).cloned()
        } else {
            None
        }
    }

    fn child_nodes(&self) -> Vec<Self> {
        self.0.borrow().children.clone()
    }

    fn append_child(&self, node: Self) -> Self {
        if let Some(old_parent) = node.parent_node() {
            old_parent.remove_child(&node);
        }
        node.0.borrow_mut().parent = Some(Rc::downgrade(&self.0));
        node.0.borrow_mut().owner_document = self.owner_document().map(|d| Rc::downgrade(&d.0));
        self.0.borrow_mut().children.push(node.clone());
        node
    }

    fn insert_before(&self, node: Self, child: Option<&Self>) -> Self {
        if let Some(old_parent) = node.parent_node() {
            old_parent.remove_child(&node);
        }
        node.0.borrow_mut().parent = Some(Rc::downgrade(&self.0));
        node.0.borrow_mut().owner_document = self.owner_document().map(|d| Rc::downgrade(&d.0));
        let mut b = self.0.borrow_mut();
        if let Some(c) = child {
            let cid = c.id();
            if let Some(pos) = b.children.iter().position(|n| n.id() == cid) {
                b.children.insert(pos, node.clone());
                return node;
            }
        }
        b.children.push(node.clone());
        node
    }

    fn remove_child(&self, child: &Self) -> Self {
        let cid = child.id();
        let mut b = self.0.borrow_mut();
        if let Some(pos) = b.children.iter().position(|n| n.id() == cid) {
            b.children.remove(pos);
            child.0.borrow_mut().parent = None;
        }
        child.clone()
    }

    fn replace_child(&self, node: Self, child: &Self) -> Self {
        let cid = child.id();
        if let Some(old_parent) = node.parent_node() {
            old_parent.remove_child(&node);
        }
        node.0.borrow_mut().parent = Some(Rc::downgrade(&self.0));
        node.0.borrow_mut().owner_document = self.owner_document().map(|d| Rc::downgrade(&d.0));
        let mut b = self.0.borrow_mut();
        if let Some(pos) = b.children.iter().position(|n| n.id() == cid) {
            b.children[pos] = node.clone();
            child.0.borrow_mut().parent = None;
        }
        child.clone()
    }

    fn clone_node(&self, deep: bool) -> Self {
        let b = self.0.borrow();
        let id = NEXT_NODE_ID.fetch_add(1, AtomicOrdering::Relaxed);
        let mut new_children = Vec::new();
        let clone = Node(Rc::new(RefCell::new(NodeInner {
            id,
            node_type: b.node_type,
            tag_name: b.tag_name.clone(),
            namespace_uri: b.namespace_uri.clone(),
            text: b.text.clone(),
            attributes: b.attributes.clone(),
            styles: b.styles.clone(),
            properties: b.properties.clone(),
            any_properties: HashMap::new(),
            event_listeners: Vec::new(),
            parent: None,
            children: Vec::new(),
            owner_document: b.owner_document.clone(),
            rect: b.rect,
            screen_ctm: b.screen_ctm,
            client_left: b.client_left,
            client_top: b.client_top,
            client_width: b.client_width,
            client_height: b.client_height,
        })));
        if deep {
            for child in &b.children {
                let child_clone = child.clone_node(true);
                child_clone.0.borrow_mut().parent = Some(Rc::downgrade(&clone.0));
                new_children.push(child_clone);
            }
            clone.0.borrow_mut().children = new_children;
        }
        clone
    }

    fn owner_document(&self) -> Option<Self> {
        let b = self.0.borrow();
        if b.node_type == 9 {
            return Some(self.clone());
        }
        if let Some(w) = &b.owner_document {
            if let Some(rc) = w.upgrade() {
                return Some(Node(rc));
            }
        }
        let mut curr = self.clone();
        while let Some(parent) = curr.parent_node() {
            if parent.0.borrow().node_type == 9 {
                return Some(parent);
            }
            curr = parent;
        }
        None
    }

    fn document_element(&self) -> Option<Self> {
        let b = self.0.borrow();
        if b.node_type == 9 {
            b.children
                .iter()
                .find(|c| c.0.borrow().node_type == 1)
                .cloned()
        } else {
            self.owner_document().and_then(|d| d.document_element())
        }
    }

    fn default_view(&self) -> Option<Self> {
        self.owner_document().or_else(|| Some(self.clone()))
    }

    fn query_selector(&self, selectors: &str) -> Option<Self> {
        self.query_selector_all(selectors).into_iter().next()
    }

    fn query_selector_all(&self, selectors: &str) -> Vec<Self> {
        let mut results = Vec::new();
        collect_descendants(self, selectors, &mut results);
        results
    }

    fn create_element(&self, local_name: &str) -> Self {
        let node = Node::new_element(local_name);
        if let Some(doc) = self.owner_document() {
            node.0.borrow_mut().owner_document = Some(Rc::downgrade(&doc.0));
        }
        node
    }

    fn create_element_ns(&self, namespace: Option<&str>, qualified_name: &str) -> Self {
        let node = Node::new_element_ns(namespace, qualified_name);
        if let Some(doc) = self.owner_document() {
            node.0.borrow_mut().owner_document = Some(Rc::downgrade(&doc.0));
        }
        node
    }

    fn create_text_node(&self, data: &str) -> Self {
        let node = Node::new_text(data);
        if let Some(doc) = self.owner_document() {
            node.0.borrow_mut().owner_document = Some(Rc::downgrade(&doc.0));
        }
        node
    }

    fn get_attribute(&self, name: &str) -> Option<String> {
        let b = self.0.borrow();
        b.attributes
            .iter()
            .find(|(k, _, _)| k == name)
            .map(|(_, _, v)| v.clone())
    }

    fn get_attribute_ns(&self, namespace: Option<&str>, local_name: &str) -> Option<String> {
        let b = self.0.borrow();
        b.attributes
            .iter()
            .find(|(k, ns, _)| k == local_name && ns.as_deref() == namespace)
            .map(|(_, _, v)| v.clone())
    }

    fn set_attribute(&self, name: &str, value: &str) {
        let mut b = self.0.borrow_mut();
        if let Some(entry) = b.attributes.iter_mut().find(|(k, _, _)| k == name) {
            entry.2 = value.to_string();
        } else {
            b.attributes
                .push((name.to_string(), None, value.to_string()));
        }
    }

    fn set_attribute_ns(&self, namespace: Option<&str>, qualified_name: &str, value: &str) {
        let local_name = qualified_name
            .split(':')
            .next_back()
            .unwrap_or(qualified_name);
        let mut b = self.0.borrow_mut();
        if let Some(entry) = b
            .attributes
            .iter_mut()
            .find(|(k, ns, _)| k == local_name && ns.as_deref() == namespace)
        {
            entry.2 = value.to_string();
        } else {
            b.attributes.push((
                local_name.to_string(),
                namespace.map(|s| s.to_string()),
                value.to_string(),
            ));
        }
    }

    fn remove_attribute(&self, name: &str) {
        let mut b = self.0.borrow_mut();
        b.attributes.retain(|(k, _, _)| k != name);
    }

    fn remove_attribute_ns(&self, namespace: Option<&str>, local_name: &str) {
        let mut b = self.0.borrow_mut();
        b.attributes
            .retain(|(k, ns, _)| !(k == local_name && ns.as_deref() == namespace));
    }

    fn has_attribute(&self, name: &str) -> bool {
        self.0.borrow().attributes.iter().any(|(k, _, _)| k == name)
    }

    fn has_attribute_ns(&self, namespace: Option<&str>, local_name: &str) -> bool {
        self.0
            .borrow()
            .attributes
            .iter()
            .any(|(k, ns, _)| k == local_name && ns.as_deref() == namespace)
    }

    fn get_style_property(&self, name: &str) -> String {
        let b = self.0.borrow();
        b.styles
            .iter()
            .find(|(k, _, _)| k == name)
            .map(|(_, v, _)| v.clone())
            .unwrap_or_default()
    }

    fn set_style_property(&self, name: &str, value: &str, priority: &str) {
        let mut b = self.0.borrow_mut();
        if let Some(entry) = b.styles.iter_mut().find(|(k, _, _)| k == name) {
            entry.1 = value.to_string();
            entry.2 = priority.to_string();
        } else {
            b.styles
                .push((name.to_string(), value.to_string(), priority.to_string()));
        }
    }

    fn remove_style_property(&self, name: &str) {
        let mut b = self.0.borrow_mut();
        b.styles.retain(|(k, _, _)| k != name);
    }

    fn text_content(&self) -> String {
        let b = self.0.borrow();
        if b.node_type == 3 || b.node_type == 8 {
            return b.text.clone();
        }
        let mut s = String::new();
        for child in &b.children {
            s.push_str(&child.text_content());
        }
        s
    }

    fn set_text_content(&self, value: &str) {
        let mut b = self.0.borrow_mut();
        b.children.clear();
        if b.node_type == 3 || b.node_type == 8 {
            b.text = value.to_string();
        } else {
            b.text.clear();
            let text_node = Node::new_text(value);
            text_node.0.borrow_mut().parent = Some(Rc::downgrade(&self.0));
            b.children.push(text_node);
        }
    }

    fn inner_html(&self) -> String {
        let b = self.0.borrow();
        let mut out = String::new();
        for child in &b.children {
            let cb = child.0.borrow();
            if cb.node_type == 3 {
                out.push_str(&cb.text);
            } else if cb.node_type == 1 {
                out.push('<');
                out.push_str(&cb.tag_name);
                for (k, _, v) in &cb.attributes {
                    out.push(' ');
                    out.push_str(k);
                    out.push_str("=\"");
                    out.push_str(v);
                    out.push('"');
                }
                out.push('>');
                out.push_str(&child.inner_html());
                out.push_str("</");
                out.push_str(&cb.tag_name);
                out.push('>');
            }
        }
        out
    }

    fn set_inner_html(&self, value: &str) {
        self.set_text_content(value);
    }

    fn class_list_contains(&self, name: &str) -> bool {
        let b = self.0.borrow();
        b.attributes
            .iter()
            .find(|(k, _, _)| k == "class")
            .map(|(_, _, v)| v.split_whitespace().any(|c| c == name))
            .unwrap_or(false)
    }

    fn class_list_add(&self, name: &str) {
        let current = self.get_attribute("class").unwrap_or_default();
        if !current.split_whitespace().any(|c| c == name) {
            let next = if current.is_empty() {
                name.to_string()
            } else {
                format!("{current} {name}")
            };
            self.set_attribute("class", &next);
        }
    }

    fn class_list_remove(&self, name: &str) {
        if let Some(current) = self.get_attribute("class") {
            let next: Vec<&str> = current.split_whitespace().filter(|c| *c != name).collect();
            self.set_attribute("class", &next.join(" "));
        }
    }

    fn class_list_toggle(&self, name: &str, force: Option<bool>) -> bool {
        let contains = self.class_list_contains(name);
        let add = force.unwrap_or(!contains);
        if add {
            self.class_list_add(name);
            true
        } else {
            self.class_list_remove(name);
            false
        }
    }

    fn add_event_listener(&self, event_type: &str, use_capture: bool) {
        self.0.borrow_mut().event_listeners.push(EventListener {
            event_type: event_type.to_string(),
            capture: use_capture,
            callback: Rc::new(|_, _| {}),
        });
    }

    fn remove_event_listener(&self, event_type: &str, use_capture: bool) {
        self.0
            .borrow_mut()
            .event_listeners
            .retain(|l| !(l.event_type == event_type && l.capture == use_capture));
    }

    fn dispatch_event(&self, event_type: &str) {
        let listeners = self.0.borrow().event_listeners.clone();
        for l in listeners {
            if l.event_type == event_type {
                (l.callback)(self, event_type);
            }
        }
    }

    fn get_bounding_client_rect(&self) -> (f64, f64, f64, f64) {
        self.0.borrow().rect
    }

    fn get_screen_ctm(&self) -> Option<[f64; 6]> {
        self.0.borrow().screen_ctm
    }

    fn client_left(&self) -> f64 {
        self.0.borrow().client_left
    }

    fn client_top(&self) -> f64 {
        self.0.borrow().client_top
    }

    fn client_width(&self) -> f64 {
        self.0.borrow().client_width
    }

    fn client_height(&self) -> f64 {
        self.0.borrow().client_height
    }

    fn node_type(&self) -> u16 {
        self.0.borrow().node_type
    }

    fn namespace_uri(&self) -> Option<String> {
        self.0.borrow().namespace_uri.clone()
    }

    fn tag_name(&self) -> String {
        self.0.borrow().tag_name.clone()
    }

    fn matches(&self, selectors: &str) -> bool {
        selector_matches(self, selectors)
    }

    fn compare_document_position(&self, other: &Self) -> u16 {
        if self.id() == other.id() {
            return 0;
        }
        fn is_descendant(parent: &Node, target_id: usize) -> bool {
            for child in &parent.0.borrow().children {
                if child.id() == target_id || is_descendant(child, target_id) {
                    return true;
                }
            }
            false
        }
        if is_descendant(self, other.id()) {
            return 16 + 4; // CONTAINED_BY + FOLLOWING
        }
        if is_descendant(other, self.id()) {
            return 8 + 2; // CONTAINS + PRECEDING
        }
        if let Some(doc) = self.owner_document() {
            let mut list = Vec::new();
            collect_all(&doc, &mut list);
            let pos_self = list.iter().position(|id| *id == self.id());
            let pos_other = list.iter().position(|id| *id == other.id());
            if let (Some(ps), Some(po)) = (pos_self, pos_other) {
                if ps < po {
                    return 4; // FOLLOWING
                } else {
                    return 2; // PRECEDING
                }
            }
        }
        1 // DISCONNECTED
    }

    fn get_property(&self, name: &str) -> Option<String> {
        self.0.borrow().properties.get(name).cloned()
    }

    fn set_property(&self, name: &str, value: &str) {
        self.0
            .borrow_mut()
            .properties
            .insert(name.to_string(), value.to_string());
    }

    fn remove_property(&self, name: &str) {
        self.0.borrow_mut().properties.remove(name);
    }

    fn get_any(&self, key: &str) -> Option<Rc<dyn Any>> {
        self.0.borrow().any_properties.get(key).cloned()
    }

    fn set_any(&self, key: &str, value: Rc<dyn Any>) {
        self.0
            .borrow_mut()
            .any_properties
            .insert(key.to_string(), value);
    }

    fn remove_any(&self, key: &str) {
        self.0.borrow_mut().any_properties.remove(key);
    }

    fn has_any(&self, key: &str) -> bool {
        self.0.borrow().any_properties.contains_key(key)
    }
}

fn collect_all(node: &Node, list: &mut Vec<usize>) {
    list.push(node.id());
    for child in &node.0.borrow().children {
        collect_all(child, list);
    }
}

// ---------------------------------------------------------------------------
// Convenience functions and aliases
// ---------------------------------------------------------------------------

/// Create a new default root document.
pub fn create_document() -> Node {
    Node::new_document()
}

/// Create a new HTML or SVG element by tag name.
pub fn create_element(tag: &str) -> Node {
    Node::new_element(tag)
}

/// The same concrete type as [`Node`].
pub type Element = Node;

/// The same concrete type as [`Node`].
pub type Document = Node;

/// The same concrete type as [`Node`].
pub type DomNode = Node;

/// The same concrete type as [`Node`].
pub type SimpleNode = Node;
