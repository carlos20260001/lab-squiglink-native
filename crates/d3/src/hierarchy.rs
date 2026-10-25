//! A port of **d3-hierarchy**, from the JavaScript of the same name.
//!
//! Translated from `d3-hierarchy/src/`, version 1.1.9, of d3 5.16.0.

use std::cell::RefCell;
use std::rc::Rc;

/// Data for a hierarchy node.
pub trait HierarchyData: Clone {
    /// The children of this datum.
    fn children(&self) -> Option<Vec<Self>> {
        None
    }
    /// The value of this datum.
    fn value(&self) -> Option<f64> {
        None
    }
}

/// A node in the hierarchy.
#[derive(Clone)]
pub struct HierarchyNode<T: HierarchyData> {
    pub data: T,
    pub depth: usize,
    pub height: usize,
    pub value: Option<f64>,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub parent: Option<Rc<RefCell<HierarchyNode<T>>>>,
    pub children: Option<Vec<Rc<RefCell<HierarchyNode<T>>>>>,
}

type HierarchyLink<T> = (Rc<RefCell<HierarchyNode<T>>>, Rc<RefCell<HierarchyNode<T>>>);

/// The JavaScript's `hierarchy`.
///
/// Builds the tree from data using a children accessor (defaults to `d.children`).
pub fn hierarchy<T: HierarchyData>(data: T) -> Rc<RefCell<HierarchyNode<T>>> {
    let root = Rc::new(RefCell::new(HierarchyNode {
        data: data.clone(),
        depth: 0,
        height: 0,
        value: data.value(),
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
        parent: None,
        children: None,
    }));

    let valued = data.value().is_some();
    let mut stack: Vec<Rc<RefCell<HierarchyNode<T>>>> = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let node_data = node.borrow().data.clone();
        if valued {
            if let Some(v) = node_data.value() {
                node.borrow_mut().value = Some(v);
            }
        }
        if let Some(childs) = node_data.children() {
            let n = childs.len();
            if n > 0 {
                let mut arr: Vec<Rc<RefCell<HierarchyNode<T>>>> = Vec::with_capacity(n);
                for i in (0..n).rev() {
                    let child = Rc::new(RefCell::new(HierarchyNode {
                        data: childs[i].clone(),
                        depth: node.borrow().depth + 1,
                        height: 0,
                        value: childs[i].value(),
                        parent: Some(node.clone()),
                        children: None,
                        x0: 0.0,
                        y0: 0.0,
                        x1: 0.0,
                        y1: 0.0,
                    }));
                    arr.push(child.clone());
                    stack.push(child);
                }
                arr.reverse();
                node.borrow_mut().children = Some(arr);
            }
        }
    }

    // computeHeight using eachBefore (post-order)
    compute_height(&root);
    root
}

fn compute_height<T: HierarchyData>(root: &Rc<RefCell<HierarchyNode<T>>>) {
    // Post-order traversal: visit children first, then parent.
    let mut stack = vec![root.clone()];
    let mut order = Vec::new();
    while let Some(node) = stack.pop() {
        order.push(node.clone());
        if let Some(children) = &node.borrow().children {
            for c in children.iter().rev() {
                stack.push(c.clone());
            }
        }
    }
    // Reset heights
    for node in &order {
        node.borrow_mut().height = 0;
    }
    // Compute from leaves up (reverse of push order: leaves pushed last are first in reverse?)
    // Actually stack pushes children in reverse so they are visited left-to-right. Post-order needs reverse of visit order? Not exactly.
    // We'll just use a recursive post-order approach via descendants.
    for node in order.into_iter().rev() {
        let h = match &node.borrow().children {
            Some(c) if !c.is_empty() => {
                let mut maxh = 0usize;
                for ch in c {
                    maxh = maxh.max(ch.borrow().height);
                }
                maxh + 1
            }
            _ => 0usize,
        };
        node.borrow_mut().height = h;
    }
}

impl<T: HierarchyData> HierarchyNode<T> {
    /// Count the leaf nodes.
    pub fn count(node: &Rc<RefCell<HierarchyNode<T>>>) -> f64 {
        Self::leaves(node).len() as f64
    }
    /// Sum with a value accessor.
    pub fn sum(node: &Rc<RefCell<HierarchyNode<T>>>, value: impl Fn(&T) -> f64) {
        let nodes = Self::descendants(node);
        let mut rev = nodes.clone();
        rev.reverse();
        for n in rev {
            let v = value(&n.borrow().data);
            let mut sum = if v.is_finite() { v } else { 0.0 };
            if let Some(children) = &n.borrow().children {
                for c in children {
                    if let Some(cv) = c.borrow().value {
                        sum += cv;
                    }
                }
            }
            n.borrow_mut().value = Some(sum);
        }
    }
    /// Descendants (breadth-first-like).
    pub fn descendants(node: &Rc<RefCell<HierarchyNode<T>>>) -> Vec<Rc<RefCell<HierarchyNode<T>>>> {
        let mut out = Vec::new();
        let mut current = vec![node.clone()];
        while !current.is_empty() {
            let mut next = Vec::new();
            for n in current {
                out.push(n.clone());
                if let Some(children) = &n.borrow().children {
                    for c in children {
                        next.push(c.clone());
                    }
                }
            }
            current = next;
        }
        out
    }

    /// Leaves.
    pub fn leaves(node: &Rc<RefCell<HierarchyNode<T>>>) -> Vec<Rc<RefCell<HierarchyNode<T>>>> {
        Self::descendants(node)
            .into_iter()
            .filter(|n| n.borrow().children.is_none())
            .collect()
    }

    /// Ancestors of this node.
    pub fn ancestors(node: &Rc<RefCell<HierarchyNode<T>>>) -> Vec<Rc<RefCell<HierarchyNode<T>>>> {
        let mut out = Vec::new();
        let mut cur = Some(node.clone());
        while let Some(n) = cur {
            out.push(n.clone());
            cur = n.borrow().parent.clone();
        }
        out
    }

    /// Links (parent-child pairs).
    pub fn links(node: &Rc<RefCell<HierarchyNode<T>>>) -> Vec<HierarchyLink<T>> {
        let mut links = Vec::new();
        let desc = Self::descendants(node);
        for n in desc {
            let p_clone = n.borrow().parent.clone();
            if let Some(p) = p_clone {
                links.push((p, n));
            } else {
                // skip root or nodes with no parent (keep for links?)
                links.push((n.clone(), n));
            }
        }
        links
    }

    /// Each before (pre-order).
    pub fn each_before<F>(&self, mut cb: F)
    where
        F: FnMut(&Self),
    {
        cb(self);
        if let Some(children) = &self.children {
            for c in children {
                c.borrow().each_before(&mut cb);
            }
        }
    }
}

pub const PHI: f64 = std::f64::consts::GOLDEN_RATIO;
