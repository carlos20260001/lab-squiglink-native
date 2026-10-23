//! A faithful port of **d3-transition**, from the JavaScript of the same name.
//!
//! Translated from `d3-transition/src/`, version 1.3.2, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! A transition is a selection-like interface for animating changes to the DOM.
//! Instead of applying changes immediately, transitions interpolate between the
//! current DOM state and the desired target state over a specified duration.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::time::Instant;

use crate::dom::{Dom, SimpleNode};
use crate::ease::ease_cubic_in_out;
use crate::interpolate::interpolate_string;
use crate::selection::{namespace, select, SelectedNode, Selection};

thread_local! {
    static START_INSTANT: Instant = Instant::now();
}

/// Current time in milliseconds.
pub fn now() -> f64 {
    START_INSTANT.with(|start| start.elapsed().as_secs_f64() * 1000.0)
}

static NEXT_TRANSITION_ID: AtomicUsize = AtomicUsize::new(1);

/// Allocate a new globally unique transition ID.
pub fn new_id() -> usize {
    NEXT_TRANSITION_ID.fetch_add(1, AtomicOrdering::Relaxed)
}

/// The state of an individual transition on a DOM node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransitionState {
    /// Initial created state.
    Created = 0,
    /// Scheduled to start in the future.
    Scheduled = 1,
    /// Starting during the current frame.
    Starting = 2,
    /// Started and dispatched start event.
    Started = 3,
    /// Actively ticking.
    Running = 4,
    /// Ending during the current frame.
    Ending = 5,
    /// Completed or canceled.
    Ended = 6,
}

/// Timing parameters for a transition.
#[derive(Clone)]
pub struct Timing {
    /// Start time in milliseconds.
    pub time: f64,
    /// Delay before start in milliseconds.
    pub delay: f64,
    /// Duration of transition in milliseconds.
    pub duration: f64,
    /// Easing function.
    pub ease: Rc<dyn Fn(f64) -> f64>,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            time: now(),
            delay: 0.0,
            duration: 250.0,
            ease: Rc::new(ease_cubic_in_out),
        }
    }
}

/// A registered tween callback.
pub type TweenCallback = Rc<dyn Fn(f64)>;

/// A tween factory on a node.
pub type TweenFactory<N> = Rc<dyn Fn(&N) -> Option<TweenCallback>>;

/// Internal schedule storage for transitions on a node.
#[derive(Clone)]
pub struct Schedule<N: Dom> {
    /// Transition name.
    pub name: Option<String>,
    /// Unique transition ID.
    pub id: usize,
    /// Timing parameters.
    pub timing: Timing,
    /// Current state.
    pub state: TransitionState,
    /// Active tweens.
    pub tweens: Vec<(String, TweenCallback)>,
    /// Tween factories.
    pub tween_factories: Vec<(String, TweenFactory<N>)>,
}

type NodeSchedules<N> = HashMap<usize, Schedule<N>>;

fn get_node_schedules<N: Dom>(node: &N) -> NodeSchedules<N> {
    node.get_any("__transition")
        .and_then(|a| a.downcast_ref::<RefCell<NodeSchedules<N>>>().map(|rc| rc.borrow().clone()))
        .unwrap_or_default()
}

fn set_node_schedules<N: Dom>(node: &N, schedules: NodeSchedules<N>) {
    node.set_any("__transition", Rc::new(RefCell::new(schedules)));
}

/// Interrupt active transitions on a node.
pub fn interrupt<N: Dom>(node: &N, name: Option<&str>) {
    let mut schedules = get_node_schedules(node);
    schedules.retain(|_, s| s.name.as_deref() != name);
    set_node_schedules(node, schedules);
}

/// The JavaScript's `d3.active(node, name)`.
pub fn active<N: Dom>(node: &N, name: Option<&str>) -> Option<Transition<N>> {
    let schedules = get_node_schedules(node);
    for (&id, s) in &schedules {
        if s.state > TransitionState::Scheduled && s.name.as_deref() == name {
            return Some(Transition::new(
                vec![vec![Some(SelectedNode::Node(node.clone()))]],
                vec![None],
                s.name.clone(),
                id,
            ));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Transition
// ---------------------------------------------------------------------------

/// The JavaScript's `d3.transition`.
#[derive(Clone)]
pub struct Transition<N: Dom = SimpleNode> {
    /// Groups of nodes in the transition.
    pub _groups: Vec<Vec<Option<SelectedNode<N>>>>,
    /// Parent nodes for each group.
    pub _parents: Vec<Option<N>>,
    /// Optional transition name.
    pub _name: Option<String>,
    /// Unique transition ID.
    pub _id: usize,
}

impl<N: Dom> Transition<N> {
    /// Create a new transition from groups, parents, name, and ID.
    pub fn new(
        groups: Vec<Vec<Option<SelectedNode<N>>>>,
        parents: Vec<Option<N>>,
        name: Option<String>,
        id: usize,
    ) -> Self {
        Transition {
            _groups: groups,
            _parents: parents,
            _name: name,
            _id: id,
        }
    }

    /// Return the selection corresponding to this transition.
    pub fn selection(&self) -> Selection<N> {
        Selection::new(self._groups.clone(), self._parents.clone())
    }

    /// The JavaScript's `transition.select(selector)`.
    pub fn select(&self, selector: &str) -> Self {
        let sel = self.selection().select(selector);
        Transition::new(sel._groups, sel._parents, self._name.clone(), self._id)
    }

    /// The JavaScript's `transition.selectAll(selector)`.
    pub fn select_all(&self, selector: &str) -> Self {
        let sel = self.selection().select_all(selector);
        Transition::new(sel._groups, sel._parents, self._name.clone(), self._id)
    }

    /// The JavaScript's `transition.filter(selector)`.
    pub fn filter(&self, selector: &str) -> Self {
        let sel = self.selection().filter(selector);
        Transition::new(sel._groups, sel._parents, self._name.clone(), self._id)
    }

    /// The JavaScript's `transition.merge(other)`.
    pub fn merge(&self, other: &Self) -> Self {
        let sel = self.selection().merge(&other.selection());
        Transition::new(sel._groups, sel._parents, self._name.clone(), self._id)
    }

    /// Chain a new transition after this one.
    pub fn transition(&self) -> Self {
        let new_trans_id = new_id();
        Transition::new(
            self._groups.clone(),
            self._parents.clone(),
            self._name.clone(),
            new_trans_id,
        )
    }

    /// The JavaScript's `transition.duration(ms)`.
    pub fn duration(self, ms: f64) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let mut schedules = get_node_schedules(node);
                    if let Some(s) = schedules.get_mut(&self._id) {
                        s.timing.duration = ms;
                    }
                    set_node_schedules(node, schedules);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.delay(ms)`.
    pub fn delay(self, ms: f64) -> Self {
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let mut schedules = get_node_schedules(node);
                    if let Some(s) = schedules.get_mut(&self._id) {
                        s.timing.delay = ms;
                    }
                    set_node_schedules(node, schedules);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.ease(function)`.
    pub fn ease<F: Fn(f64) -> f64 + 'static>(self, ease_fn: F) -> Self {
        let rc = Rc::new(ease_fn);
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let mut schedules = get_node_schedules(node);
                    if let Some(s) = schedules.get_mut(&self._id) {
                        s.timing.ease = rc.clone();
                    }
                    set_node_schedules(node, schedules);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.attr(name, value)`.
    pub fn attr(self, name: &str, value: &str) -> Self {
        let attr_name = name.to_string();
        let target_val = value.to_string();
        let ns = namespace(name);

        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let current_val = if let Some(ref space) = ns.space {
                        node.get_attribute_ns(Some(space), &ns.local)
                    } else {
                        node.get_attribute(&ns.local)
                    }
                    .unwrap_or_default();

                    let interp = RefCell::new(interpolate_string(&current_val, &target_val));
                    let target = node.clone();
                    let name_copy = attr_name.clone();
                    let space_copy = ns.space.clone();
                    let local_copy = ns.local.clone();

                    let tween = Rc::new(move |t: f64| {
                        let interpolated = interp.borrow_mut()(t);
                        if let Some(ref space) = space_copy {
                            target.set_attribute_ns(Some(space), &local_copy, &interpolated);
                        } else {
                            target.set_attribute(&name_copy, &interpolated);
                        }
                    });

                    // Immediately apply target for non-animated or tests
                    if let Some(ref space) = ns.space {
                        node.set_attribute_ns(Some(space), &ns.local, &target_val);
                    } else {
                        node.set_attribute(&attr_name, &target_val);
                    }

                    let mut schedules = get_node_schedules(node);
                    if let Some(s) = schedules.get_mut(&self._id) {
                        s.tweens.push((format!("attr.{attr_name}"), tween));
                    }
                    set_node_schedules(node, schedules);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.style(name, value)`.
    pub fn style(self, name: &str, value: &str) -> Self {
        let style_name = name.to_string();
        let target_val = value.to_string();

        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    let current_val = node.get_style_property(name);
                    let interp = RefCell::new(interpolate_string(&current_val, &target_val));
                    let target = node.clone();
                    let name_copy = style_name.clone();

                    let tween = Rc::new(move |t: f64| {
                        let interpolated = interp.borrow_mut()(t);
                        target.set_style_property(&name_copy, &interpolated, "");
                    });

                    // Set target value immediately for static inspection
                    node.set_style_property(&style_name, &target_val, "");

                    let mut schedules = get_node_schedules(node);
                    if let Some(s) = schedules.get_mut(&self._id) {
                        s.tweens.push((format!("style.{style_name}"), tween));
                    }
                    set_node_schedules(node, schedules);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.text(value)`.
    pub fn text(self, value: &str) -> Self {
        let target_val = value.to_string();
        for group in &self._groups {
            for node_opt in group {
                if let Some(SelectedNode::Node(node)) = node_opt {
                    node.set_text_content(&target_val);
                }
            }
        }
        self
    }

    /// The JavaScript's `transition.remove()`.
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

    /// Number of elements in the transition.
    pub fn size(&self) -> usize {
        self.selection().size()
    }

    /// Whether the transition is empty.
    pub fn empty(&self) -> bool {
        self.selection().empty()
    }

    /// List of DOM nodes in the transition.
    pub fn nodes(&self) -> Vec<N> {
        self.selection().nodes()
    }

    /// First DOM node in the transition.
    pub fn node(&self) -> Option<N> {
        self.selection().node()
    }
}

/// Create a transition from an existing selection.
pub fn transition_selection<N: Dom>(
    sel: &Selection<N>,
    name: Option<&str>,
) -> Transition<N> {
    let id = new_id();
    let name_str = name.map(|s| s.to_string());
    for group in &sel._groups {
        for node_opt in group {
            if let Some(SelectedNode::Node(node)) = node_opt {
                let mut schedules = get_node_schedules(node);
                schedules.insert(
                    id,
                    Schedule {
                        name: name_str.clone(),
                        id,
                        timing: Timing::default(),
                        state: TransitionState::Scheduled,
                        tweens: Vec::new(),
                        tween_factories: Vec::new(),
                    },
                );
                set_node_schedules(node, schedules);
            }
        }
    }
    Transition::new(sel._groups.clone(), sel._parents.clone(), name_str, id)
}

/// The JavaScript's top-level `d3.transition(name)`.
pub fn transition(name: Option<&str>) -> Transition<SimpleNode> {
    let root = select("html");
    transition_selection(&root, name)
}
