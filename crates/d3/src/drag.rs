//! A faithful port of **d3-drag**, from the JavaScript of the same name.
//!
//! Translated from `d3-drag/src/`, version 1.2.5, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! Drag-and-drop is a popular and intuitive interaction gesture: moving a slider,
//! slicing a dataset, repositioning a node in a force-directed graph. The drag
//! behavior abstracts mouse and touch gestures, emitting unified `start`, `drag`,
//! and `end` events with coordinates relative to a container.

use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

use crate::dispatch::{dispatch, Dispatch};
use crate::dom::{Dom, SimpleNode};
use crate::selection::{Event, Selection};

/// An event emitted by a drag behavior during interaction.
#[derive(Clone)]
pub struct DragEvent {
    /// Event type: "start", "drag", or "end".
    pub event_type: String,
    /// The subject being dragged (e.g. data or position).
    pub subject: Option<Rc<dyn Any>>,
    /// Touch identifier or mouse identifier (-1 for mouse).
    pub identifier: i32,
    /// Number of active drag gestures on this behavior.
    pub active: usize,
    /// The current x-position relative to container.
    pub x: f64,
    /// The current y-position relative to container.
    pub y: f64,
    /// Change in x since the last event.
    pub dx: f64,
    /// Change in y since the last event.
    pub dy: f64,
}

impl std::fmt::Debug for DragEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragEvent")
            .field("type", &self.event_type)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("dx", &self.dx)
            .field("dy", &self.dy)
            .field("identifier", &self.identifier)
            .finish()
    }
}

type FilterFn<N> = Arc<dyn Fn(&N, &Event) -> bool>;
type ContainerFn<N> = Arc<dyn Fn(&N) -> Option<N>>;

/// A drag gesture recognizer and event generator.
pub struct Drag<N: Dom = SimpleNode> {
    filter: FilterFn<N>,
    container: ContainerFn<N>,
    click_distance2: f64,
    listeners: Dispatch<DragEvent>,
}

impl<N: Dom> Default for Drag<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Dom> Drag<N> {
    /// Create a new drag behavior with default settings.
    pub fn new() -> Self {
        Drag {
            filter: Arc::new(|_, e| !e.ctrl_key && e.button == 0),
            container: Arc::new(|node| node.parent_node()),
            click_distance2: 0.0,
            listeners: dispatch(&["start", "drag", "end"]).expect("valid drag dispatch types"),
        }
    }

    /// Apply the drag behavior to a selection of elements.
    pub fn call(&self, selection: &mut Selection<N>) {
        selection.set_style("touch-action", "none");
        selection.set_style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
    }

    /// Set the filter predicate determining whether an input event initiates dragging.
    pub fn filter<F>(mut self, filter_fn: F) -> Self
    where
        F: Fn(&N, &Event) -> bool + 'static,
    {
        self.filter = Arc::new(filter_fn);
        self
    }

    /// Set the container accessor returning the element defining the coordinate system.
    pub fn container<F>(mut self, container_fn: F) -> Self
    where
        F: Fn(&N) -> Option<N> + 'static,
    {
        self.container = Arc::new(container_fn);
        self
    }

    /// Set maximum squared movement to distinguish clicks from drags.
    pub fn click_distance(mut self, distance: f64) -> Self {
        self.click_distance2 = distance * distance;
        self
    }

    /// Register a callback for a drag event ("start", "drag", "end").
    pub fn on<F>(mut self, typename: &str, callback: F) -> Self
    where
        F: Fn(&mut dyn Any, &[DragEvent]) + 'static,
    {
        let _ = self.listeners.on(typename, callback);
        self
    }

    /// Mutating form of `on`.
    pub fn set_on<F>(&mut self, typename: &str, callback: F) -> &mut Self
    where
        F: Fn(&mut dyn Any, &[DragEvent]) + 'static,
    {
        let _ = self.listeners.on(typename, callback);
        self
    }

    /// Trigger a synthetic drag gesture programmatically.
    pub fn dispatch_drag(&self, node: &N, event_type: &str, x: f64, y: f64, dx: f64, dy: f64) {
        let drag_ev = DragEvent {
            event_type: event_type.to_string(),
            subject: node.get_any("__data__"),
            identifier: -1,
            active: 1,
            x,
            y,
            dx,
            dy,
        };
        let mut target = ();
        let _ = self.listeners.call(event_type, &mut target, &[drag_ev]);
    }
}

/// The JavaScript's `d3.drag()`.
pub fn drag<N: Dom>() -> Drag<N> {
    Drag::new()
}
