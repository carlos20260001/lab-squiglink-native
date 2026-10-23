//! A port of **d3-force**, from the JavaScript of the same name.
//!
//! Translated from `d3-force/src/*.js`, version 1.2.1, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read differently,
//! the JavaScript is right.
//!
//! A force is a function that, given the current strength `alpha` of the
//! simulation, pushes the nodes a little. The simulation owns the nodes and
//! calls every force it has been given, then moves the nodes by their
//! velocity. The port keeps that shape: a [`Force`] is a trait with an
//! [`initialize`](Force::initialize) and an [`apply`](Force::apply), and a
//! [`Simulation`] owns the nodes and the forces.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::arith;
use crate::collection;
use crate::js;
use crate::quadtree;

/// A node of a simulation.
///
/// The JavaScript does not define a node type of its own; a node is any
/// object with an `x` and a `y` and a velocity. The port keeps those, and
/// the fixed position (`fx`, `fy`) and the `index` the simulation gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The horizontal position.
    pub x: f64,
    /// The vertical position.
    pub y: f64,
    /// The horizontal velocity.
    pub vx: f64,
    /// The vertical velocity.
    pub vy: f64,
    /// The fixed horizontal position, when the node is pinned.
    pub fx: Option<f64>,
    /// The fixed vertical position, when the node is pinned.
    pub fy: Option<f64>,
    /// The index the simulation gives the node.
    pub index: usize,
}

impl Node {
    /// A node at a position, with no velocity and no fixed position.
    pub fn new(x: f64, y: f64) -> Self {
        Node {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            fx: None,
            fy: None,
            index: 0,
        }
    }
}

/// A link between two nodes, by their indices.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// The index of the source node.
    pub source: usize,
    /// The index of the target node.
    pub target: usize,
    /// The index the simulation gives the link.
    pub index: usize,
}

/// A function that reads a number out of a node.
pub type NodeFn = Box<dyn Fn(&Node, usize, &[Node]) -> f64>;

/// A function that reads a number out of a link.
pub type LinkFn = Box<dyn Fn(&Link, usize, &[Link]) -> f64>;

/// A value that may be a number or a function of a node.
pub trait IntoNodeFn {
    /// The function the value becomes.
    fn into_node_fn(self) -> NodeFn;
}

impl IntoNodeFn for f64 {
    fn into_node_fn(self) -> NodeFn {
        Box::new(move |_, _, _| self)
    }
}

impl<F> IntoNodeFn for F
where
    F: Fn(&Node, usize, &[Node]) -> f64 + 'static,
{
    fn into_node_fn(self) -> NodeFn {
        Box::new(self)
    }
}

/// A value that may be a number or a function of a link.
pub trait IntoLinkFn {
    /// The function the value becomes.
    fn into_link_fn(self) -> LinkFn;
}

impl IntoLinkFn for f64 {
    fn into_link_fn(self) -> LinkFn {
        Box::new(move |_, _, _| self)
    }
}

impl<F> IntoLinkFn for F
where
    F: Fn(&Link, usize, &[Link]) -> f64 + 'static,
{
    fn into_link_fn(self) -> LinkFn {
        Box::new(self)
    }
}

/// The JavaScript's `constant`, which gives back a function returning the
/// number it was given.
pub fn constant(value: f64) -> impl Fn(&Node, usize, &[Node]) -> f64 + 'static {
    move |_, _, _| value
}

/// A small linear congruential generator, which is the port's stand-in
/// for `Math.random` in [`jiggle`].
struct Lcg {
    state: u32,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Lcg { state: seed }
    }

    fn next(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        f64::from(self.state) / 4_294_967_296.0
    }
}

thread_local! {
    static JIGGLE_SOURCE: std::cell::RefCell<Lcg> =
        std::cell::RefCell::new(Lcg::new(1));
}

/// The JavaScript's `jiggle`, which gives a tiny random number.
pub fn jiggle() -> f64 {
    JIGGLE_SOURCE.with(|source| (source.borrow_mut().next() - 0.5) * 1e-6)
}

/// Reset the stream behind [`jiggle`], so that a scenario starts from a
/// known state. This is not part of d3's own surface; it is here so that
/// a test can run on its own.
pub fn reset_jiggle(seed: u32) {
    JIGGLE_SOURCE.with(|source| *source.borrow_mut() = Lcg::new(seed));
}

/// A force, which the simulation calls with the current `alpha`.
pub trait Force {
    /// The JavaScript's `force.initialize`, which gives the force its nodes.
    fn initialize(&mut self, nodes: &[Node]);
    /// The JavaScript's `force(alpha)`, which pushes the nodes.
    fn apply(&mut self, alpha: f64, nodes: &mut [Node]);
}

/// The state a quadtree node carries in the Barnes-Hut sums.
#[derive(Clone, Copy, Default)]
struct QuadState {
    value: f64,
    x: f64,
    y: f64,
    r: f64,
}

fn child_bounds(i: usize, x0: f64, y0: f64, x1: f64, y1: f64) -> (f64, f64, f64, f64) {
    let xm = (x0 + x1) / 2.0;
    let ym = (y0 + y1) / 2.0;
    match i {
        0 => (x0, y0, xm, ym),
        1 => (xm, y0, x1, ym),
        2 => (x0, ym, xm, y1),
        _ => (xm, ym, x1, y1),
    }
}

// ---------------------------------------------------------------------------
// Center
// ---------------------------------------------------------------------------

/// The JavaScript's `forceCenter`.
#[derive(Clone)]
pub struct Center {
    x: f64,
    y: f64,
    nodes: Vec<Node>,
}

impl Center {
    /// A center force at a point.
    pub fn new(x: f64, y: f64) -> Self {
        Center {
            x,
            y,
            nodes: Vec::new(),
        }
    }

    /// The JavaScript's `force.x` setter, for the center's x.
    pub fn set_x(&mut self, value: f64) -> &mut Self {
        self.x = value;
        self
    }

    /// The JavaScript's `force.x` getter, for the center's x.
    pub fn get_x(&self) -> f64 {
        self.x
    }

    /// The JavaScript's `force.y` setter, for the center's y.
    pub fn set_y(&mut self, value: f64) -> &mut Self {
        self.y = value;
        self
    }

    /// The JavaScript's `force.y` getter, for the center's y.
    pub fn get_y(&self) -> f64 {
        self.y
    }
}

impl Force for Center {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
    }

    fn apply(&mut self, _alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if n == 0 {
            return;
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        for node in nodes.iter() {
            sx = arith::add(sx, node.x);
            sy = arith::add(sy, node.y);
        }
        let sx = arith::divide(sx, n as f64) - self.x;
        let sy = arith::divide(sy, n as f64) - self.y;
        for node in nodes.iter_mut() {
            node.x -= sx;
            node.y -= sy;
        }
    }
}

/// The JavaScript's `forceCenter`.
pub fn force_center(x: f64, y: f64) -> Center {
    Center::new(x, y)
}

// ---------------------------------------------------------------------------
// X
// ---------------------------------------------------------------------------

/// The JavaScript's `forceX`.
pub struct ForceX {
    x: NodeFn,
    strength: NodeFn,
    nodes: Vec<Node>,
    strengths: Vec<f64>,
    xz: Vec<f64>,
}

impl ForceX {
    /// A force pulling towards an x.
    pub fn new<X: IntoNodeFn>(x: X) -> Self {
        ForceX {
            x: x.into_node_fn(),
            strength: Box::new(|_, _, _| 0.1),
            nodes: Vec::new(),
            strengths: Vec::new(),
            xz: Vec::new(),
        }
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength<S: IntoNodeFn>(&mut self, strength: S) -> &mut Self {
        self.strength = strength.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> &NodeFn {
        &self.strength
    }

    /// The JavaScript's `force.x` setter.
    pub fn set_x<X: IntoNodeFn>(&mut self, x: X) -> &mut Self {
        self.x = x.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.x` getter.
    pub fn get_x(&self) -> &NodeFn {
        &self.x
    }
}

impl Force for ForceX {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        self.strengths.resize(n, 0.0);
        self.xz.resize(n, 0.0);
        for i in 0..n {
            let node = &nodes[i];
            let target = (self.x)(node, i, nodes);
            self.xz[i] = js::number(target);
            self.strengths[i] = if self.xz[i].is_nan() {
                0.0
            } else {
                (self.strength)(node, i, nodes)
            };
        }
    }

    fn apply(&mut self, alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if self.strengths.len() != n {
            self.initialize(nodes);
        }
        for i in 0..n {
            nodes[i].vx += (self.xz[i] - nodes[i].x) * self.strengths[i] * alpha;
        }
    }
}

/// The JavaScript's `forceX`.
pub fn force_x<X: IntoNodeFn>(x: X) -> ForceX {
    ForceX::new(x)
}

// ---------------------------------------------------------------------------
// Y
// ---------------------------------------------------------------------------

/// The JavaScript's `forceY`.
pub struct ForceY {
    y: NodeFn,
    strength: NodeFn,
    nodes: Vec<Node>,
    strengths: Vec<f64>,
    yz: Vec<f64>,
}

impl ForceY {
    /// A force pulling towards a y.
    pub fn new<Y: IntoNodeFn>(y: Y) -> Self {
        ForceY {
            y: y.into_node_fn(),
            strength: Box::new(|_, _, _| 0.1),
            nodes: Vec::new(),
            strengths: Vec::new(),
            yz: Vec::new(),
        }
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength<S: IntoNodeFn>(&mut self, strength: S) -> &mut Self {
        self.strength = strength.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> &NodeFn {
        &self.strength
    }

    /// The JavaScript's `force.y` setter.
    pub fn set_y<Y: IntoNodeFn>(&mut self, y: Y) -> &mut Self {
        self.y = y.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.y` getter.
    pub fn get_y(&self) -> &NodeFn {
        &self.y
    }
}

impl Force for ForceY {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        self.strengths.resize(n, 0.0);
        self.yz.resize(n, 0.0);
        for i in 0..n {
            let node = &nodes[i];
            let target = (self.y)(node, i, nodes);
            self.yz[i] = js::number(target);
            self.strengths[i] = if self.yz[i].is_nan() {
                0.0
            } else {
                (self.strength)(node, i, nodes)
            };
        }
    }

    fn apply(&mut self, alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if self.strengths.len() != n {
            self.initialize(nodes);
        }
        for i in 0..n {
            nodes[i].vy += (self.yz[i] - nodes[i].y) * self.strengths[i] * alpha;
        }
    }
}

/// The JavaScript's `forceY`.
pub fn force_y<Y: IntoNodeFn>(y: Y) -> ForceY {
    ForceY::new(y)
}

// ---------------------------------------------------------------------------
// Many-body
// ---------------------------------------------------------------------------

/// The JavaScript's `forceManyBody`.
pub struct ManyBody {
    strength: NodeFn,
    nodes: Vec<Node>,
    strengths: Vec<f64>,
    distance_min2: f64,
    distance_max2: f64,
    theta2: f64,
}

impl ManyBody {
    /// A many-body force.
    pub fn new() -> Self {
        ManyBody {
            strength: Box::new(|_, _, _| -30.0),
            nodes: Vec::new(),
            strengths: Vec::new(),
            distance_min2: 1.0,
            distance_max2: f64::INFINITY,
            theta2: 0.81,
        }
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength<S: IntoNodeFn>(&mut self, strength: S) -> &mut Self {
        self.strength = strength.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> &NodeFn {
        &self.strength
    }

    /// The JavaScript's `force.distanceMin` setter.
    pub fn distance_min(&mut self, value: f64) -> &mut Self {
        self.distance_min2 = value * value;
        self
    }

    /// The JavaScript's `force.distanceMin` getter.
    pub fn get_distance_min(&self) -> f64 {
        self.distance_min2.sqrt()
    }

    /// The JavaScript's `force.distanceMax` setter.
    pub fn distance_max(&mut self, value: f64) -> &mut Self {
        self.distance_max2 = value * value;
        self
    }

    /// The JavaScript's `force.distanceMax` getter.
    pub fn get_distance_max(&self) -> f64 {
        if self.distance_max2.is_infinite() {
            f64::INFINITY
        } else {
            self.distance_max2.sqrt()
        }
    }

    /// The JavaScript's `force.theta` setter.
    pub fn theta(&mut self, value: f64) -> &mut Self {
        self.theta2 = value * value;
        self
    }

    /// The JavaScript's `force.theta` getter.
    pub fn get_theta(&self) -> f64 {
        self.theta2.sqrt()
    }
}

impl Force for ManyBody {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        self.strengths.resize(n, 0.0);
        for i in 0..n {
            let node = &nodes[i];
            self.strengths[node.index] = (self.strength)(node, i, nodes);
        }
    }

    fn apply(&mut self, alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if self.strengths.len() != n {
            self.initialize(nodes);
        }
        let strengths = self.strengths.clone();
        let distance_min2 = self.distance_min2;
        let distance_max2 = self.distance_max2;
        let theta2 = self.theta2;

        // Build the quadtree over the nodes' current positions.
        let points: Vec<(usize, f64, f64)> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (i, node.x, node.y))
            .collect();
        let mut tree = quadtree::Quadtree::<(usize, f64, f64)>::with_accessors(
            |d: &(usize, f64, f64)| d.1,
            |d: &(usize, f64, f64)| d.2,
        );
        tree.add_all(&points);

        let (x0, y0, x1, y1) = match tree.extent() {
            Some(ext) => (ext.0 .0, ext.0 .1, ext.1 .0, ext.1 .1),
            None => return,
        };

        // Accumulate the sums.
        let mut states = HashMap::new();
        if let Some(root) = tree.root() {
            accumulate_many(root, x0, y0, x1, y1, &strengths, nodes, &mut states);
        }

        // Apply to every node.
        if let Some(root) = tree.root() {
            for target in 0..n {
                visit_many(
                    root,
                    x0,
                    y0,
                    x1,
                    y1,
                    target,
                    alpha,
                    nodes,
                    &strengths,
                    theta2,
                    distance_min2,
                    distance_max2,
                    &states,
                );
            }
        }
    }
}

fn accumulate_many(
    node: &quadtree::Node<(usize, f64, f64)>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    strengths: &[f64],
    nodes: &[Node],
    states: &mut HashMap<usize, QuadState>,
) {
    let ptr = node as *const _ as usize;
    match node {
        quadtree::Node::Branch(children) => {
            let xm = (x0 + x1) / 2.0;
            let ym = (y0 + y1) / 2.0;
            let mut strength = 0.0;
            let mut weight = 0.0;
            let mut sx = 0.0;
            let mut sy = 0.0;
            for i in 0..4 {
                let child_ptr = children[i].as_ref().map(|c| c.as_ref() as *const _ as usize);
                if let Some(child_box) = &children[i] {
                    let child = child_box.as_ref();
                    let (cx0, cy0, cx1, cy1) = child_bounds(i, x0, y0, x1, y1);
                    accumulate_many(child, cx0, cy0, cx1, cy1, strengths, nodes, states);
                    if let Some(cptr) = child_ptr {
                        if let Some(st) = states.get(&cptr).copied() {
                            let c = st.value.abs();
                            if c != 0.0 {
                                strength += st.value;
                                weight += c;
                                sx += c * st.x;
                                sy += c * st.y;
                            }
                        }
                    }
                }
            }
            let qx = if weight != 0.0 {
                sx / weight
            } else {
                f64::NAN
            };
            let qy = if weight != 0.0 {
                sy / weight
            } else {
                f64::NAN
            };
            states.insert(
                ptr,
                QuadState {
                    value: strength,
                    x: qx,
                    y: qy,
                    r: 0.0,
                },
            );
        }
        quadtree::Node::Leaf(leaf) => {
            let mut strength = 0.0;
            let mut q = Some(leaf);
            while let Some(l) = q {
                let idx = nodes[l.data.0].index;
                strength += strengths[idx];
                q = l.next.as_deref();
            }
            let x = nodes[leaf.data.0].x;
            let y = nodes[leaf.data.0].y;
            states.insert(
                ptr,
                QuadState {
                    value: strength,
                    x,
                    y,
                    r: 0.0,
                },
            );
        }
    }
}

fn visit_many(
    quad: &quadtree::Node<(usize, f64, f64)>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    target: usize,
    alpha: f64,
    nodes: &mut [Node],
    strengths: &[f64],
    theta2: f64,
    distance_min2: f64,
    distance_max2: f64,
    states: &HashMap<usize, QuadState>,
) {
    let ptr = quad as *const _ as usize;
    let st = match states.get(&ptr).copied() {
        Some(st) => st,
        None => return,
    };
    if st.value == 0.0 {
        return;
    }
    let x = st.x - nodes[target].x;
    let y = st.y - nodes[target].y;
    let w = x1 - x0;
    let l = x * x + y * y;

    if w * w / theta2 < l {
        if l < distance_max2 {
            let mut x = x;
            let mut y = y;
            let mut l = l;
            if x == 0.0 {
                x = jiggle();
                l += x * x;
            }
            if y == 0.0 {
                y = jiggle();
                l += y * y;
            }
            if l < distance_min2 {
                l = (distance_min2 * l).sqrt();
            }
            nodes[target].vx += x * st.value * alpha / l;
            nodes[target].vy += y * st.value * alpha / l;
        }
        return;
    }

    let is_branch = matches!(quad, quadtree::Node::Branch(_));
    if is_branch || l >= distance_max2 {
        // descend if it is a branch
    } else {
        // leaf: process directly
        if let quadtree::Node::Leaf(leaf) = quad {
            let first_data = leaf.data;
            let mut count = 0usize;
            let mut q = Some(leaf);
            let mut indices = Vec::new();
            while let Some(lf) = q {
                indices.push(lf.data);
                count += 1;
                q = lf.next.as_deref();
            }
            let has_other = count > 1 || nodes[first_data.0].index != nodes[target].index;
            let mut x = x;
            let mut y = y;
            let mut l = l;
            if has_other {
                if x == 0.0 {
                    x = jiggle();
                    l += x * x;
                }
                if y == 0.0 {
                    y = jiggle();
                    l += y * y;
                }
                if l < distance_min2 {
                    l = (distance_min2 * l).sqrt();
                }
            }
            for &data_pos in &indices {
                if nodes[data_pos.0].index != nodes[target].index {
                    let w = strengths[nodes[data_pos.0].index] * alpha / l;
                    nodes[target].vx += x * w;
                    nodes[target].vy += y * w;
                }
            }
        }
        return;
    }

    if let quadtree::Node::Branch(children) = quad {
        let xm = (x0 + x1) / 2.0;
        let ym = (y0 + y1) / 2.0;
        for i in 0..4 {
            if let Some(child_box) = &children[i] {
                let child = child_box.as_ref();
                let (cx0, cy0, cx1, cy1) = child_bounds(i, x0, y0, x1, y1);
                visit_many(
                    child,
                    cx0,
                    cy0,
                    cx1,
                    cy1,
                    target,
                    alpha,
                    nodes,
                    strengths,
                    theta2,
                    distance_min2,
                    distance_max2,
                    states,
                );
            }
        }
    }
}

/// The JavaScript's `forceManyBody`.
pub fn force_many_body() -> ManyBody {
    ManyBody::new()
}

// ---------------------------------------------------------------------------
// Collide
// ---------------------------------------------------------------------------

/// The JavaScript's `forceCollide`.
pub struct Collide {
    radius: NodeFn,
    strength: f64,
    iterations: usize,
    nodes: Vec<Node>,
    radii: Vec<f64>,
}

impl Collide {
    /// A collide force.
    pub fn new<R: IntoNodeFn>(radius: R) -> Self {
        Collide {
            radius: radius.into_node_fn(),
            strength: 1.0,
            iterations: 1,
            nodes: Vec::new(),
            radii: Vec::new(),
        }
    }

    /// The JavaScript's `force.radius` setter.
    pub fn radius<R: IntoNodeFn>(&mut self, radius: R) -> &mut Self {
        self.radius = radius.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.radius` getter.
    pub fn get_radius(&self) -> &NodeFn {
        &self.radius
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength(&mut self, value: f64) -> &mut Self {
        self.strength = value;
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> f64 {
        self.strength
    }

    /// The JavaScript's `force.iterations` setter.
    pub fn iterations(&mut self, value: usize) -> &mut Self {
        self.iterations = value;
        self
    }

    /// The JavaScript's `force.iterations` getter.
    pub fn get_iterations(&self) -> usize {
        self.iterations
    }
}

impl Force for Collide {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        self.radii.resize(n, 0.0);
        for i in 0..n {
            let node = &nodes[i];
            self.radii[node.index] = (self.radius)(node, i, nodes);
        }
    }

    fn apply(&mut self, _alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if self.radii.len() != n {
            self.initialize(nodes);
        }
        let radii = self.radii.clone();
        let strength = self.strength;
        let iterations = self.iterations;

        for _ in 0..iterations {
            // Build the quadtree over the nodes' predicted positions.
            let points: Vec<(usize, f64, f64)> = nodes
                .iter()
                .enumerate()
                .map(|(i, node)| (i, node.x + node.vx, node.y + node.vy))
                .collect();
            let mut tree = quadtree::Quadtree::<(usize, f64, f64)>::with_accessors(
                |d: &(usize, f64, f64)| d.1,
                |d: &(usize, f64, f64)| d.2,
            );
            tree.add_all(&points);

            let (x0, y0, x1, y1) = match tree.extent() {
                Some(ext) => (ext.0 .0, ext.0 .1, ext.1 .0, ext.1 .1),
                None => continue,
            };

            // Prepare the radii on the tree.
            let mut states = HashMap::new();
            if let Some(root) = tree.root() {
                prepare_collide(root, &radii, nodes, &mut states);
            }

            if let Some(root) = tree.root() {
                for i in 0..n {
                    let node_index = nodes[i].index;
                    let ri = radii[node_index];
                    let ri2 = ri * ri;
                    let xi = nodes[i].x + nodes[i].vx;
                    let yi = nodes[i].y + nodes[i].vy;
                    collide_visit(
                        root,
                        x0,
                        y0,
                        x1,
                        y1,
                        i,
                        ri,
                        ri2,
                        xi,
                        yi,
                        strength,
                        &radii,
                        nodes,
                        &states,
                    );
                }
            }
        }
    }
}

fn prepare_collide(
    node: &quadtree::Node<(usize, f64, f64)>,
    radii: &[f64],
    nodes: &[Node],
    states: &mut HashMap<usize, QuadState>,
) {
    let ptr = node as *const _ as usize;
    match node {
        quadtree::Node::Branch(children) => {
            let mut r = 0.0;
            for i in 0..4 {
                if let Some(child_box) = &children[i] {
                    let child = child_box.as_ref();
                    prepare_collide(child, radii, nodes, states);
                    let cptr = child as *const _ as usize;
                    if let Some(st) = states.get(&cptr).copied() {
                        if st.r > r {
                            r = st.r;
                        }
                    }
                }
            }
            states.insert(ptr, QuadState { r, ..QuadState::default() });
        }
        quadtree::Node::Leaf(leaf) => {
            let data_pos = leaf.data;
            let node_index = nodes[data_pos.0].index;
            let r = radii[node_index];
            states.insert(ptr, QuadState { r, ..QuadState::default() });
        }
    }
}

fn collide_visit(
    quad: &quadtree::Node<(usize, f64, f64)>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    target: usize,
    ri: f64,
    ri2: f64,
    xi: f64,
    yi: f64,
    strength: f64,
    radii: &[f64],
    nodes: &mut [Node],
    states: &HashMap<usize, QuadState>,
) {
    let ptr = quad as *const _ as usize;
    let st = match states.get(&ptr).copied() {
        Some(st) => st,
        None => return,
    };
    let rj = st.r;
    let r = ri + rj;

    let is_leaf = matches!(quad, quadtree::Node::Leaf(_));
    if is_leaf {
        if let quadtree::Node::Leaf(leaf) = quad {
            let data_pos = leaf.data;
            let data_index = nodes[data_pos.0].index;
            if data_index > nodes[target].index {
                let x = xi - (nodes[data_pos.0].x + nodes[data_pos.0].vx);
                let y = yi - (nodes[data_pos.0].y + nodes[data_pos.0].vy);
                let mut l = x * x + y * y;
                if l < r * r {
                    let mut x = x;
                    let mut y = y;
                    if x == 0.0 {
                        x = jiggle();
                        l += x * x;
                    }
                    if y == 0.0 {
                        y = jiggle();
                        l += y * y;
                    }
                    let sqrtl = l.sqrt();
                    let factor = (r - sqrtl) / sqrtl * strength;
                    let x = x * factor;
                    let y = y * factor;
                    let ratio = (rj * rj) / (ri2 + rj * rj);
                    nodes[target].vx += x * ratio;
                    nodes[target].vy += y * ratio;
                    let rest = 1.0 - ratio;
                    nodes[data_pos.0].vx -= x * rest;
                    nodes[data_pos.0].vy -= y * rest;
                }
            }
        }
        return;
    }

    if x0 > xi + r || x1 < xi - r || y0 > yi + r || y1 < yi - r {
        return;
    }

    if let quadtree::Node::Branch(children) = quad {
        let xm = (x0 + x1) / 2.0;
        let ym = (y0 + y1) / 2.0;
        for i in 0..4 {
            if let Some(child_box) = &children[i] {
                let child = child_box.as_ref();
                let (cx0, cy0, cx1, cy1) = child_bounds(i, x0, y0, x1, y1);
                collide_visit(
                    child,
                    cx0,
                    cy0,
                    cx1,
                    cy1,
                    target,
                    ri,
                    ri2,
                    xi,
                    yi,
                    strength,
                    radii,
                    nodes,
                    states,
                );
            }
        }
    }
}

/// The JavaScript's `forceCollide`.
pub fn force_collide<R: IntoNodeFn>(radius: R) -> Collide {
    Collide::new(radius)
}

// ---------------------------------------------------------------------------
// Link
// ---------------------------------------------------------------------------

/// The JavaScript's `forceLink`.
pub struct LinkForce {
    links: Vec<Link>,
    id: Box<dyn Fn(&Node, usize, &[Node]) -> String>,
    strength: Option<LinkFn>,
    strengths: Vec<f64>,
    distance: Option<LinkFn>,
    distances: Vec<f64>,
    nodes: Vec<Node>,
    bias: Vec<f64>,
    iterations: usize,
}

impl LinkForce {
    /// A link force.
    pub fn new(links: Vec<Link>) -> Self {
        LinkForce {
            links,
            id: Box::new(|node, _, _| node.index.to_string()),
            strength: None,
            strengths: Vec::new(),
            distance: None,
            distances: Vec::new(),
            nodes: Vec::new(),
            bias: Vec::new(),
            iterations: 1,
        }
    }

    /// The JavaScript's `force.links` setter.
    pub fn links(&mut self, links: Vec<Link>) -> &mut Self {
        self.links = links;
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.links` getter.
    pub fn get_links(&self) -> &[Link] {
        &self.links
    }

    /// The JavaScript's `force.id` setter.
    pub fn id<F>(&mut self, id: F) -> &mut Self
    where
        F: Fn(&Node, usize, &[Node]) -> String + 'static,
    {
        self.id = Box::new(id);
        self
    }

    /// The JavaScript's `force.id` getter.
    pub fn get_id(&self) -> &dyn Fn(&Node, usize, &[Node]) -> String {
        &*self.id
    }

    /// The JavaScript's `force.iterations` setter.
    pub fn iterations(&mut self, value: usize) -> &mut Self {
        self.iterations = value;
        self
    }

    /// The JavaScript's `force.iterations` getter.
    pub fn get_iterations(&self) -> usize {
        self.iterations
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength<S: IntoLinkFn>(&mut self, strength: S) -> &mut Self {
        self.strength = Some(strength.into_link_fn());
        self.initialize_strength();
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> Option<&LinkFn> {
        self.strength.as_ref()
    }

    /// The JavaScript's `force.distance` setter.
    pub fn distance<D: IntoLinkFn>(&mut self, distance: D) -> &mut Self {
        self.distance = Some(distance.into_link_fn());
        self.initialize_distance();
        self
    }

    /// The JavaScript's `force.distance` getter.
    pub fn get_distance(&self) -> Option<&LinkFn> {
        self.distance.as_ref()
    }

    fn initialize_strength(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let m = self.links.len();
        self.strengths.resize(m, 0.0);
        for i in 0..m {
            let link = &self.links[i];
            self.strengths[i] = match &self.strength {
                Some(f) => f(link, i, &self.links),
                None => {
                    let count_source = self.count_for(link.source);
                    let count_target = self.count_for(link.target);
                    let min = if count_source < count_target {
                        count_source
                    } else {
                        count_target
                    };
                    1.0 / min as f64
                }
            };
        }
    }

    fn initialize_distance(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let m = self.links.len();
        self.distances.resize(m, 0.0);
        for i in 0..m {
            let link = &self.links[i];
            self.distances[i] = match &self.distance {
                Some(f) => f(link, i, &self.links),
                None => 30.0,
            };
        }
    }

    fn count_for(&self, node_index: usize) -> usize {
        // Count links touching this node.
        let mut count = 0usize;
        for link in &self.links {
            if link.source == node_index || link.target == node_index {
                count += 1;
            }
        }
        if count == 0 {
            1
        } else {
            count
        }
    }
}

impl Force for LinkForce {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        let m = self.links.len();

        // Resolve links (they are already indices) and count degrees.
        let mut count = vec![0usize; n];
        for i in 0..m {
            self.links[i].index = i;
            let s = self.links[i].source;
            let t = self.links[i].target;
            if s < n {
                count[s] += 1;
            }
            if t < n {
                count[t] += 1;
            }
        }

        self.bias.resize(m, 0.0);
        for i in 0..m {
            let s = self.links[i].source;
            let t = self.links[i].target;
            let cs = count[s];
            let ct = count[t];
            self.bias[i] = cs as f64 / (cs + ct) as f64;
        }

        self.strengths.resize(m, 0.0);
        self.distances.resize(m, 0.0);
        self.initialize_strength();
        self.initialize_distance();
    }

    fn apply(&mut self, alpha: f64, nodes: &mut [Node]) {
        for _ in 0..self.iterations {
            let m = self.links.len();
            for i in 0..m {
                let source = self.links[i].source;
                let target = self.links[i].target;
                let x = (nodes[target].x + nodes[target].vx)
                    - (nodes[source].x + nodes[source].vx);
                let x = if x == 0.0 || x.is_nan() {
                    jiggle()
                } else {
                    x
                };
                let y = (nodes[target].y + nodes[target].vy)
                    - (nodes[source].y + nodes[source].vy);
                let y = if y == 0.0 || y.is_nan() {
                    jiggle()
                } else {
                    y
                };
                let l = (x * x + y * y).sqrt();
                let l = (l - self.distances[i]) / l * alpha * self.strengths[i];
                let x = x * l;
                let y = y * l;
                let b = self.bias[i];
                nodes[target].vx -= x * b;
                nodes[target].vy -= y * b;
                nodes[source].vx += x * (1.0 - b);
                nodes[source].vy += y * (1.0 - b);
            }
        }
    }
}

/// The JavaScript's `forceLink`.
pub fn force_link(links: Vec<Link>) -> LinkForce {
    LinkForce::new(links)
}

// ---------------------------------------------------------------------------
// Radial
// ---------------------------------------------------------------------------

/// The JavaScript's `forceRadial`.
pub struct Radial {
    radius: NodeFn,
    strength: NodeFn,
    x: f64,
    y: f64,
    nodes: Vec<Node>,
    strengths: Vec<f64>,
    radiuses: Vec<f64>,
}

impl Radial {
    /// A radial force.
    pub fn new<R: IntoNodeFn>(radius: R, x: f64, y: f64) -> Self {
        Radial {
            radius: radius.into_node_fn(),
            strength: Box::new(|_, _, _| 0.1),
            x,
            y,
            nodes: Vec::new(),
            strengths: Vec::new(),
            radiuses: Vec::new(),
        }
    }

    /// The JavaScript's `force.radius` setter.
    pub fn radius<R: IntoNodeFn>(&mut self, radius: R) -> &mut Self {
        self.radius = radius.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.radius` getter.
    pub fn get_radius(&self) -> &NodeFn {
        &self.radius
    }

    /// The JavaScript's `force.strength` setter.
    pub fn strength<S: IntoNodeFn>(&mut self, strength: S) -> &mut Self {
        self.strength = strength.into_node_fn();
        self.initialize(&self.nodes.clone());
        self
    }

    /// The JavaScript's `force.strength` getter.
    pub fn get_strength(&self) -> &NodeFn {
        &self.strength
    }

    /// The JavaScript's `force.x` setter.
    pub fn set_x(&mut self, value: f64) -> &mut Self {
        self.x = value;
        self
    }

    /// The JavaScript's `force.x` getter.
    pub fn get_x(&self) -> f64 {
        self.x
    }

    /// The JavaScript's `force.y` setter.
    pub fn set_y(&mut self, value: f64) -> &mut Self {
        self.y = value;
        self
    }

    /// The JavaScript's `force.y` getter.
    pub fn get_y(&self) -> f64 {
        self.y
    }
}

impl Force for Radial {
    fn initialize(&mut self, nodes: &[Node]) {
        self.nodes = nodes.to_vec();
        let n = nodes.len();
        self.strengths.resize(n, 0.0);
        self.radiuses.resize(n, 0.0);
        for i in 0..n {
            let node = &nodes[i];
            let radius = (self.radius)(node, i, nodes);
            self.radiuses[i] = js::number(radius);
            self.strengths[i] = if self.radiuses[i].is_nan() {
                0.0
            } else {
                (self.strength)(node, i, nodes)
            };
        }
    }

    fn apply(&mut self, alpha: f64, nodes: &mut [Node]) {
        let n = nodes.len();
        if self.strengths.len() != n {
            self.initialize(nodes);
        }
        for i in 0..n {
            let node = &mut nodes[i];
            let dx = node.x - self.x;
            let dx = if dx == 0.0 || dx.is_nan() {
                1e-6
            } else {
                dx
            };
            let dy = node.y - self.y;
            let dy = if dy == 0.0 || dy.is_nan() {
                1e-6
            } else {
                dy
            };
            let r = (dx * dx + dy * dy).sqrt();
            let k = (self.radiuses[i] - r) * self.strengths[i] * alpha / r;
            node.vx += dx * k;
            node.vy += dy * k;
        }
    }
}

/// The JavaScript's `forceRadial`.
pub fn force_radial<R: IntoNodeFn>(radius: R, x: f64, y: f64) -> Radial {
    Radial::new(radius, x, y)
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

/// The JavaScript's `forceSimulation`.
pub struct Simulation {
    nodes: Vec<Node>,
    alpha: f64,
    alpha_min: f64,
    alpha_decay: f64,
    alpha_target: f64,
    velocity_decay: f64,
    forces: Vec<(String, Box<dyn Force>)>,
    on_tick: Option<Box<dyn FnMut()>>,
    on_end: Option<Box<dyn FnMut()>>,
}

impl Simulation {
    /// A simulation over some nodes.
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut sim = Simulation {
            nodes: Vec::new(),
            alpha: 1.0,
            alpha_min: 0.001,
            alpha_decay: 1.0 - 0.001_f64.powf(1.0 / 300.0),
            alpha_target: 0.0,
            velocity_decay: 0.6,
            forces: Vec::new(),
            on_tick: None,
            on_end: None,
        };
        sim.nodes(nodes);
        sim
    }

    /// The JavaScript's `simulation.nodes` setter.
    pub fn set_nodes(&mut self, nodes: Vec<Node>) -> &mut Self {
        self.nodes(nodes)
    }

    /// The JavaScript's `simulation.nodes` getter.
    pub fn get_nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The JavaScript's `simulation.nodes`.
    pub fn nodes(&mut self, nodes: Vec<Node>) -> &mut Self {
        let mut nodes = nodes;
        for i in 0..nodes.len() {
            let node = &mut nodes[i];
            node.index = i;
            if let Some(fx) = node.fx {
                node.x = fx;
            }
            if let Some(fy) = node.fy {
                node.y = fy;
            }
            if node.x.is_nan() || node.y.is_nan() {
                let radius = 10.0 * (i as f64).sqrt();
                let angle = i as f64 * PI * (3.0 - 5.0_f64.sqrt());
                node.x = radius * angle.cos();
                node.y = radius * angle.sin();
            }
            if node.vx.is_nan() || node.vy.is_nan() {
                node.vx = 0.0;
                node.vy = 0.0;
            }
        }
        self.nodes = nodes;
        for (_, force) in self.forces.iter_mut() {
            force.initialize(&self.nodes);
        }
        self
    }

    /// The JavaScript's `simulation.alpha` setter.
    pub fn set_alpha(&mut self, value: f64) -> &mut Self {
        self.alpha = value;
        self
    }

    /// The JavaScript's `simulation.alpha` getter.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// The JavaScript's `simulation.alphaMin` setter.
    pub fn set_alpha_min(&mut self, value: f64) -> &mut Self {
        self.alpha_min = value;
        self
    }

    /// The JavaScript's `simulation.alphaMin` getter.
    pub fn alpha_min(&self) -> f64 {
        self.alpha_min
    }

    /// The JavaScript's `simulation.alphaDecay` setter.
    pub fn set_alpha_decay(&mut self, value: f64) -> &mut Self {
        self.alpha_decay = value;
        self
    }

    /// The JavaScript's `simulation.alphaDecay` getter.
    pub fn alpha_decay(&self) -> f64 {
        self.alpha_decay
    }

    /// The JavaScript's `simulation.alphaTarget` setter.
    pub fn set_alpha_target(&mut self, value: f64) -> &mut Self {
        self.alpha_target = value;
        self
    }

    /// The JavaScript's `simulation.alphaTarget` getter.
    pub fn alpha_target(&self) -> f64 {
        self.alpha_target
    }

    /// The JavaScript's `simulation.velocityDecay` setter.
    ///
    /// The JavaScript stores `1 - value` and returns `1 - stored`; the port
    /// stores the multiplier directly.
    pub fn set_velocity_decay(&mut self, value: f64) -> &mut Self {
        self.velocity_decay = 1.0 - value;
        self
    }

    /// The JavaScript's `simulation.velocityDecay` getter.
    pub fn velocity_decay(&self) -> f64 {
        1.0 - self.velocity_decay
    }

    /// The JavaScript's `simulation.force` setter.
    pub fn set_force(&mut self, name: &str, force: Box<dyn Force>) -> &mut Self {
        let mut force = force;
        force.initialize(&self.nodes);
        if let Some(slot) = self.forces.iter_mut().find(|(n, _)| n == name) {
            slot.1 = force;
        } else {
            self.forces.push((name.to_owned(), force));
        }
        self
    }

    /// The JavaScript's `simulation.force` getter.
    pub fn get_force(&self, name: &str) -> Option<&dyn Force> {
        self.forces
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| &**f)
    }

    /// The JavaScript's `simulation.force(name, null)`.
    pub fn remove_force(&mut self, name: &str) -> &mut Self {
        self.forces.retain(|(n, _)| n != name);
        self
    }

    /// The JavaScript's `simulation.tick`.
    pub fn tick(&mut self, iterations: Option<usize>) -> &mut Self {
        let iterations = iterations.unwrap_or(1);
        for _ in 0..iterations {
            self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;
            let alpha = self.alpha;

            let mut forces = std::mem::take(&mut self.forces);
            for (_, force) in forces.iter_mut() {
                force.apply(alpha, &mut self.nodes);
            }
            self.forces = forces;

            for node in self.nodes.iter_mut() {
                if node.fx.is_none() {
                    node.vx *= self.velocity_decay;
                    node.x += node.vx;
                } else {
                    node.x = node.fx.unwrap();
                    node.vx = 0.0;
                }
                if node.fy.is_none() {
                    node.vy *= self.velocity_decay;
                    node.y += node.vy;
                } else {
                    node.y = node.fy.unwrap();
                    node.vy = 0.0;
                }
            }
        }
        if let Some(callback) = self.on_tick.as_mut() {
            callback();
        }
        if self.alpha < self.alpha_min {
            if let Some(callback) = self.on_end.as_mut() {
                callback();
            }
        }
        self
    }

    /// The JavaScript's `simulation.restart`.
    pub fn restart(&mut self) -> &mut Self {
        self
    }

    /// The JavaScript's `simulation.stop`.
    pub fn stop(&mut self) -> &mut Self {
        self
    }

    /// The JavaScript's `simulation.find`.
    pub fn find(&self, x: f64, y: f64, radius: Option<f64>) -> Option<&Node> {
        let mut radius2 = match radius {
            Some(r) => r * r,
            None => f64::INFINITY,
        };
        let mut closest: Option<&Node> = None;
        for node in &self.nodes {
            let dx = x - node.x;
            let dy = y - node.y;
            let d2 = dx * dx + dy * dy;
            if d2 < radius2 {
                closest = Some(node);
                radius2 = d2;
            }
        }
        closest
    }

    /// Register a callback for the tick event.
    pub fn on_tick<F: FnMut() + 'static>(&mut self, callback: F) -> &mut Self {
        self.on_tick = Some(Box::new(callback));
        self
    }

    /// Register a callback for the end event.
    pub fn on_end<F: FnMut() + 'static>(&mut self, callback: F) -> &mut Self {
        self.on_end = Some(Box::new(callback));
        self
    }
}

/// The JavaScript's `forceSimulation`.
pub fn force_simulation(nodes: Vec<Node>) -> Simulation {
    Simulation::new(nodes)
}

/// The JavaScript's `x`, exported from the simulation module.
pub fn x(d: &Node) -> f64 {
    d.x
}

/// The JavaScript's `y`, exported from the simulation module.
pub fn y(d: &Node) -> f64 {
    d.y
}
