//! A port of **d3-quadtree**, from the JavaScript of the same name.
//!
//! Translated from `d3-quadtree/src/`, version 1.0.7, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.
//!
//! A quadtree subdivides a square into four until every point has a
//! leaf of its own. Points that fall on the same spot share a leaf as a
//! short chain, which is the JavaScript's own way of keeping two
//! measurements that are the same place apart.

use std::sync::Arc;

/// A function that reads a coordinate out of a datum.
pub type Accessor<T> = Arc<dyn Fn(&T) -> f64>;

/// A datum with a point in it, for the tree's default accessors.
///
/// The JavaScript's default `x` reads `d[0]` and its default `y` reads
/// `d[1]`; the port asks the datum for the same two numbers.
pub trait Point {
    /// The JavaScript's `d[0]`.
    fn x(&self) -> f64;
    /// The JavaScript's `d[1]`.
    fn y(&self) -> f64;
}

impl Point for [f64; 2] {
    fn x(&self) -> f64 {
        self[0]
    }

    fn y(&self) -> f64 {
        self[1]
    }
}

impl Point for (f64, f64) {
    fn x(&self) -> f64 {
        self.0
    }

    fn y(&self) -> f64 {
        self.1
    }
}

impl Point for Vec<f64> {
    fn x(&self) -> f64 {
        self.first().copied().unwrap_or(f64::NAN)
    }

    fn y(&self) -> f64 {
        self.get(1).copied().unwrap_or(f64::NAN)
    }
}

/// A leaf of the tree, and the points that share its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Leaf<T> {
    /// The datum at this place.
    pub data: T,
    /// The next datum at the same place, which the JavaScript keeps in
    /// `leaf.next`.
    pub next: Option<Box<Leaf<T>>>,
}

impl<T> Leaf<T> {
    /// A leaf of one datum, with no other datum at the same place.
    pub fn new(data: T) -> Self {
        Self { data, next: None }
    }
}

/// A node of the tree: either a leaf or a branch of four quadrants.
#[derive(Clone, Debug, PartialEq)]
pub enum Node<T> {
    /// A place with one or more data at it.
    Leaf(Leaf<T>),
    /// A branch, whose four children are the quadrants in the
    /// JavaScript's order: bottom-left, top-left, bottom-right,
    /// top-right.
    Branch(Box<[Option<Box<Node<T>>>; 4]>),
}

impl<T> Node<T> {
    /// Whether the node is a leaf.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf(_))
    }

    /// Whether the node is a branch.
    pub fn is_branch(&self) -> bool {
        matches!(self, Node::Branch(_))
    }

    /// The four children of a branch, or `None` for a leaf.
    pub fn children(&self) -> Option<&[Option<Box<Node<T>>>; 4]> {
        match self {
            Node::Branch(children) => Some(children),
            Node::Leaf(_) => None,
        }
    }
}

/// A quadtree, which the JavaScript calls a `Quadtree`.
pub struct Quadtree<T> {
    x: Accessor<T>,
    y: Accessor<T>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    root: Option<Box<Node<T>>>,
}

impl<T: Point> Default for Quadtree<T> {
    fn default() -> Self {
        Self::with_accessors(|d: &T| d.x(), |d: &T| d.y())
    }
}

impl<T: Point> Quadtree<T> {
    /// The JavaScript's `quadtree()`: a tree with the default `x` and
    /// `y`, and no extent.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Quadtree<T> {
    /// A tree with accessors of its own, which is the JavaScript's
    /// `quadtree(nodes, x, y)` before any nodes are added.
    pub fn with_accessors<X, Y>(x: X, y: Y) -> Self
    where
        X: Fn(&T) -> f64 + 'static,
        Y: Fn(&T) -> f64 + 'static,
    {
        Self {
            x: Arc::new(x),
            y: Arc::new(y),
            x0: f64::NAN,
            y0: f64::NAN,
            x1: f64::NAN,
            y1: f64::NAN,
            root: None,
        }
    }

    /// The JavaScript's `quadtree.x` getter.
    pub fn x(&self) -> &Accessor<T> {
        &self.x
    }

    /// The JavaScript's `quadtree.x` setter.
    pub fn set_x<F>(&mut self, x: F) -> &mut Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.x = Arc::new(x);
        self
    }

    /// The JavaScript's `quadtree.y` getter.
    pub fn y(&self) -> &Accessor<T> {
        &self.y
    }

    /// The JavaScript's `quadtree.y` setter.
    pub fn set_y<F>(&mut self, y: F) -> &mut Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.y = Arc::new(y);
        self
    }

    /// The JavaScript's `quadtree.extent` getter.
    ///
    /// The JavaScript gives `undefined` when the tree has no extent,
    /// and the port gives `None`.
    pub fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        if self.x0.is_nan() {
            None
        } else {
            Some(((self.x0, self.y0), (self.x1, self.y1)))
        }
    }

    /// The JavaScript's `quadtree.extent` setter, which covers the
    /// given extent.
    pub fn set_extent(&mut self, extent: ((f64, f64), (f64, f64))) -> &mut Self {
        self.cover(extent.0 .0, extent.0 .1);
        self.cover(extent.1 .0, extent.1 .1)
    }

    /// The JavaScript's `quadtree.root`, which gives the root node.
    pub fn root(&self) -> Option<&Node<T>> {
        self.root.as_deref()
    }

    /// The JavaScript's `quadtree.cover`, which grows the extent until
    /// it covers the point.
    pub fn cover(&mut self, x: f64, y: f64) -> &mut Self {
        if x.is_nan() || y.is_nan() {
            return self;
        }
        let mut x0 = self.x0;
        let mut y0 = self.y0;
        let mut x1 = self.x1;
        let mut y1 = self.y1;
        if x0.is_nan() {
            x0 = x.floor();
            y0 = y.floor();
            x1 = x0 + 1.0;
            y1 = y0 + 1.0;
        } else {
            let mut z = x1 - x0;
            let branch = self.root.as_deref().is_some_and(Node::is_branch);
            let mut node = self.root.take();
            while x0 > x || x >= x1 || y0 > y || y >= y1 {
                let i = ((y < y0) as usize) << 1 | ((x < x0) as usize);
                let mut parent: [Option<Box<Node<T>>>; 4] = [None, None, None, None];
                parent[i] = node;
                node = Some(Box::new(Node::Branch(Box::new(parent))));
                z *= 2.0;
                match i {
                    0 => {
                        x1 = x0 + z;
                        y1 = y0 + z;
                    }
                    1 => {
                        x0 = x1 - z;
                        y1 = y0 + z;
                    }
                    2 => {
                        x1 = x0 + z;
                        y0 = y1 - z;
                    }
                    _ => {
                        x0 = x1 - z;
                        y0 = y1 - z;
                    }
                }
            }
            if branch {
                self.root = node;
            } else {
                self.root = node.and_then(|node| match *node {
                    Node::Branch(mut children) => children[0].take(),
                    Node::Leaf(leaf) => Some(Box::new(Node::Leaf(leaf))),
                });
            }
        }
        self.x0 = x0;
        self.y0 = y0;
        self.x1 = x1;
        self.y1 = y1;
        self
    }

    /// The JavaScript's `quadtree.add`, which adds one datum.
    pub fn add(&mut self, data: T) -> &mut Self {
        let x = (self.x)(&data);
        let y = (self.y)(&data);
        self.cover(x, y);
        self.add_point(x, y, data);
        self
    }

    fn add_point(&mut self, x: f64, y: f64, data: T) {
        if x.is_nan() || y.is_nan() {
            return;
        }
        let root = self.root.take();
        self.root = Some(Box::new(insert(
            root, self.x0, self.y0, self.x1, self.y1, x, y, data, &self.x, &self.y,
        )));
    }

    /// The JavaScript's `quadtree.addAll`, which adds a list of data.
    ///
    /// The JavaScript stores the data themselves; the port keeps its
    /// own copies, and so asks the datum to be cloned.
    pub fn add_all(&mut self, data: &[T]) -> &mut Self
    where
        T: Clone,
    {
        let mut points: Vec<Option<(f64, f64)>> = Vec::with_capacity(data.len());
        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for datum in data {
            let x = (self.x)(datum);
            let y = (self.y)(datum);
            if x.is_nan() || y.is_nan() {
                points.push(None);
                continue;
            }
            points.push(Some((x, y)));
            if x < x0 {
                x0 = x;
            }
            if x > x1 {
                x1 = x;
            }
            if y < y0 {
                y0 = y;
            }
            if y > y1 {
                y1 = y;
            }
        }
        if x0 > x1 || y0 > y1 {
            return self;
        }
        self.cover(x0, y0);
        self.cover(x1, y1);
        for (datum, point) in data.iter().zip(points) {
            if let Some((x, y)) = point {
                self.add_point(x, y, datum.clone());
            }
        }
        self
    }

    /// The JavaScript's `quadtree.data`, which gives the data in the
    /// order the tree visits them.
    pub fn data(&self) -> Vec<&T> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            collect_data(root, &mut out);
        }
        out
    }

    /// The JavaScript's `quadtree.size`, which counts the data.
    pub fn size(&self) -> usize {
        self.data().len()
    }

    /// The JavaScript's `quadtree.find`, which finds the datum nearest
    /// to a point, within a radius.
    pub fn find(&self, x: f64, y: f64, radius: Option<f64>) -> Option<&T> {
        let mut x0 = self.x0;
        let mut y0 = self.y0;
        let mut x3 = self.x1;
        let mut y3 = self.y1;
        let mut quads = Vec::new();
        let mut radius = radius.unwrap_or(f64::INFINITY);
        if let Some(root) = &self.root {
            quads.push(Quad::new(root, x0, y0, x3, y3));
        } else {
            return None;
        }
        if radius != f64::INFINITY {
            x0 = x - radius;
            y0 = y - radius;
            x3 = x + radius;
            y3 = y + radius;
            radius *= radius;
        }
        let mut data = None;
        while let Some(q) = quads.pop() {
            let node = q.node;
            let (x1, y1, x2, y2) = (q.x0, q.y0, q.x1, q.y1);
            if x1 > x3 || y1 > y3 || x2 < x0 || y2 < y0 {
                continue;
            }
            match node {
                Node::Branch(children) => {
                    let xm = (x1 + x2) / 2.0;
                    let ym = (y1 + y2) / 2.0;
                    if let Some(child) = &children[3] {
                        quads.push(Quad::new(child, xm, ym, x2, y2));
                    }
                    if let Some(child) = &children[2] {
                        quads.push(Quad::new(child, x1, ym, xm, y2));
                    }
                    if let Some(child) = &children[1] {
                        quads.push(Quad::new(child, xm, y1, x2, ym));
                    }
                    if let Some(child) = &children[0] {
                        quads.push(Quad::new(child, x1, y1, xm, ym));
                    }
                    let i = ((y >= ym) as usize) << 1 | ((x >= xm) as usize);
                    if i != 0 {
                        let last = quads.len() - 1;
                        quads.swap(last, last - i);
                    }
                }
                Node::Leaf(leaf) => {
                    let dx = x - (self.x)(&leaf.data);
                    let dy = y - (self.y)(&leaf.data);
                    let d2 = dx * dx + dy * dy;
                    if d2 < radius {
                        let d = radius.sqrt();
                        radius = d2;
                        x0 = x - d;
                        y0 = y - d;
                        x3 = x + d;
                        y3 = y + d;
                        data = Some(&leaf.data);
                    }
                }
            }
        }
        data
    }

    /// The JavaScript's `quadtree.remove`, which removes one datum.
    pub fn remove(&mut self, data: &T) -> &mut Self
    where
        T: PartialEq,
    {
        let x = (self.x)(data);
        let y = (self.y)(data);
        if x.is_nan() || y.is_nan() {
            return self;
        }
        let root = self.root.take();
        self.root = remove(
            root, self.x0, self.y0, self.x1, self.y1, x, y, data, &self.x, &self.y,
        );
        self
    }

    /// The JavaScript's `quadtree.removeAll`, which removes a list of
    /// data.
    pub fn remove_all(&mut self, data: &[T]) -> &mut Self
    where
        T: PartialEq,
    {
        for datum in data {
            self.remove(datum);
        }
        self
    }

    /// The JavaScript's `quadtree.copy`, which makes a copy of the tree.
    pub fn copy(&self) -> Self
    where
        T: Clone,
    {
        Self {
            x: Arc::clone(&self.x),
            y: Arc::clone(&self.y),
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
            root: self.root.clone(),
        }
    }

    /// The JavaScript's `quadtree.visit`, which visits each node before
    /// its children.
    ///
    /// The callback returns whether it has handled the node itself, and
    /// so whether the walk should not descend into it.
    pub fn visit<F>(&self, mut callback: F) -> &Self
    where
        F: FnMut(&Node<T>, f64, f64, f64, f64) -> bool,
    {
        if let Some(root) = &self.root {
            visit(root, self.x0, self.y0, self.x1, self.y1, &mut callback);
        }
        self
    }

    /// The JavaScript's `quadtree.visitAfter`, which visits each node
    /// after its children.
    pub fn visit_after<F>(&self, mut callback: F) -> &Self
    where
        F: FnMut(&Node<T>, f64, f64, f64, f64),
    {
        if let Some(root) = &self.root {
            visit_after(root, self.x0, self.y0, self.x1, self.y1, &mut callback);
        }
        self
    }
}

/// A tree of data with a point in each, using the default accessors.
pub fn quadtree<T: Point + Clone>(nodes: &[T]) -> Quadtree<T> {
    let mut tree = Quadtree::new();
    tree.add_all(nodes);
    tree
}

/// The JavaScript's `add`, which adds a point at a known coordinate.
fn insert<T>(
    node: Option<Box<Node<T>>>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x: f64,
    y: f64,
    data: T,
    accessor_x: &Accessor<T>,
    accessor_y: &Accessor<T>,
) -> Node<T> {
    let Some(node) = node else {
        return Node::Leaf(Leaf::new(data));
    };
    match *node {
        Node::Branch(mut children) => {
            let xm = (x0 + x1) / 2.0;
            let ym = (y0 + y1) / 2.0;
            let right = x >= xm;
            let bottom = y >= ym;
            let i = ((bottom as usize) << 1) | (right as usize);
            let (child_x0, child_x1, child_y0, child_y1) = if right {
                (
                    xm,
                    x1,
                    if bottom { ym } else { y0 },
                    if bottom { y1 } else { ym },
                )
            } else {
                (
                    x0,
                    xm,
                    if bottom { ym } else { y0 },
                    if bottom { y1 } else { ym },
                )
            };
            children[i] = Some(Box::new(insert(
                children[i].take(),
                child_x0,
                child_y0,
                child_x1,
                child_y1,
                x,
                y,
                data,
                accessor_x,
                accessor_y,
            )));
            Node::Branch(children)
        }
        Node::Leaf(leaf) => {
            let xp = (accessor_x)(&leaf.data);
            let yp = (accessor_y)(&leaf.data);
            if x == xp && y == yp {
                let mut new_leaf = Leaf::new(data);
                new_leaf.next = Some(Box::new(leaf));
                return Node::Leaf(new_leaf);
            }
            let old = Node::Leaf(leaf);
            split(x0, y0, x1, y1, x, y, xp, yp, old, data)
        }
    }
}

/// The JavaScript's split, which makes branches until the old point and
/// the new one are in different quadrants.
fn split<T>(
    mut x0: f64,
    mut y0: f64,
    mut x1: f64,
    mut y1: f64,
    x: f64,
    y: f64,
    xp: f64,
    yp: f64,
    old: Node<T>,
    data: T,
) -> Node<T> {
    let mut child = None;
    loop {
        let xm = (x0 + x1) / 2.0;
        let ym = (y0 + y1) / 2.0;
        let right = x >= xm;
        let bottom = y >= ym;
        let i = ((bottom as usize) << 1) | (right as usize);
        let j = (((yp >= ym) as usize) << 1) | ((xp >= xm) as usize);
        let mut children = [None, None, None, None];
        if let Some(node) = child.take() {
            children[i] = Some(Box::new(node));
        }
        let branch = Node::Branch(Box::new(children));
        if i == j {
            child = Some(branch);
            if right {
                x0 = xm;
            } else {
                x1 = xm;
            }
            if bottom {
                y0 = ym;
            } else {
                y1 = ym;
            }
            continue;
        }
        let mut children = match branch {
            Node::Branch(children) => children,
            Node::Leaf(_) => unreachable!(),
        };
        children[j] = Some(Box::new(old));
        children[i] = Some(Box::new(Node::Leaf(Leaf::new(data))));
        return Node::Branch(children);
    }
}

/// The JavaScript's `remove`, which finds a datum and takes it out,
/// collapsing a parent that is left with one leaf.
#[allow(clippy::only_used_in_recursion)]
fn remove<T>(
    node: Option<Box<Node<T>>>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x: f64,
    y: f64,
    data: &T,
    accessor_x: &Accessor<T>,
    accessor_y: &Accessor<T>,
) -> Option<Box<Node<T>>>
where
    T: PartialEq,
{
    let node = node?;
    match *node {
        Node::Leaf(leaf) => remove_leaf(leaf, data),
        Node::Branch(mut children) => {
            let xm = (x0 + x1) / 2.0;
            let ym = (y0 + y1) / 2.0;
            let right = x >= xm;
            let bottom = y >= ym;
            let i = ((bottom as usize) << 1) | (right as usize);
            let (child_x0, child_x1) = if right { (xm, x1) } else { (x0, xm) };
            let (child_y0, child_y1) = if bottom { (ym, y1) } else { (y0, ym) };
            let before = children[i].is_some();
            let child = remove(
                children[i].take(),
                child_x0,
                child_y0,
                child_x1,
                child_y1,
                x,
                y,
                data,
                accessor_x,
                accessor_y,
            );
            let removed = before && child.is_none() || child.is_some();
            children[i] = child;
            if !removed {
                return Some(Box::new(Node::Branch(children)));
            }
            collapse(children)
        }
    }
}

/// The JavaScript's removal of a datum from a leaf chain.
fn remove_leaf<T>(mut head: Leaf<T>, data: &T) -> Option<Box<Node<T>>>
where
    T: PartialEq,
{
    if &head.data == data {
        return head.next.take().map(|leaf| Box::new(Node::Leaf(*leaf)));
    }
    let mut link = &mut head.next;
    while link.is_some() {
        let target = link.as_ref().unwrap().data == *data;
        if target {
            let next = link.as_mut().unwrap();
            *link = next.next.take();
            return Some(Box::new(Node::Leaf(head)));
        }
        let next = link.as_mut().unwrap();
        link = &mut next.next;
    }
    Some(Box::new(Node::Leaf(head)))
}

/// The JavaScript's collapse of a parent with one child that is a leaf.
fn collapse<T>(mut children: Box<[Option<Box<Node<T>>>; 4]>) -> Option<Box<Node<T>>> {
    let mut one_idx = None;
    let mut count = 0;
    for (i, child) in children.iter().enumerate() {
        if child.is_some() {
            count += 1;
            one_idx = Some(i);
        }
    }
    if count == 1 {
        let i = one_idx.unwrap();
        let is_branch = children[i].as_ref().map(|c| c.is_branch()).unwrap_or(false);
        if !is_branch {
            return children[i].take();
        }
    }
    Some(Box::new(Node::Branch(children)))
}

/// The JavaScript's `data` walk.
fn collect_data<'a, T>(node: &'a Node<T>, out: &mut Vec<&'a T>) {
    match node {
        Node::Branch(children) => {
            for child in children.iter().flatten() {
                collect_data(child, out);
            }
        }
        Node::Leaf(leaf) => {
            let mut leaf = Some(leaf);
            while let Some(current) = leaf {
                out.push(&current.data);
                leaf = current.next.as_deref();
            }
        }
    }
}

/// The JavaScript's `Quad`, for the walk in `find` and `visitAfter`.
struct Quad<'a, T> {
    node: &'a Node<T>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl<'a, T> Quad<'a, T> {
    fn new(node: &'a Node<T>, x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            node,
            x0,
            y0,
            x1,
            y1,
        }
    }
}

/// The JavaScript's `visit`.
fn visit<T, F>(node: &Node<T>, x0: f64, y0: f64, x1: f64, y1: f64, callback: &mut F)
where
    F: FnMut(&Node<T>, f64, f64, f64, f64) -> bool,
{
    if callback(node, x0, y0, x1, y1) {
        return;
    }
    if let Node::Branch(children) = node {
        let xm = (x0 + x1) / 2.0;
        let ym = (y0 + y1) / 2.0;
        if let Some(child) = &children[0] {
            visit(child, x0, y0, xm, ym, callback);
        }
        if let Some(child) = &children[1] {
            visit(child, xm, y0, x1, ym, callback);
        }
        if let Some(child) = &children[2] {
            visit(child, x0, ym, xm, y1, callback);
        }
        if let Some(child) = &children[3] {
            visit(child, xm, ym, x1, y1, callback);
        }
    }
}

/// The JavaScript's `visitAfter`.
fn visit_after<T, F>(node: &Node<T>, x0: f64, y0: f64, x1: f64, y1: f64, callback: &mut F)
where
    F: FnMut(&Node<T>, f64, f64, f64, f64),
{
    if let Node::Branch(children) = node {
        let xm = (x0 + x1) / 2.0;
        let ym = (y0 + y1) / 2.0;
        if let Some(child) = &children[0] {
            visit_after(child, x0, y0, xm, ym, callback);
        }
        if let Some(child) = &children[1] {
            visit_after(child, xm, y0, x1, ym, callback);
        }
        if let Some(child) = &children[2] {
            visit_after(child, x0, ym, xm, y1, callback);
        }
        if let Some(child) = &children[3] {
            visit_after(child, xm, ym, x1, y1, callback);
        }
    }
    callback(node, x0, y0, x1, y1);
}
