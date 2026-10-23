//! A port of **d3-voronoi**, from the JavaScript of the same name.
//!
//! Translated from `d3-voronoi/src/`, version 1.1.2, of d3 5.16.0.
//! The JavaScript is the authority; where the two could be read
//! differently, the JavaScript is right.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::js;

/// The JavaScript's `epsilon`, the tolerance used to round the sites.
pub const EPSILON: f64 = 1e-6;

/// The JavaScript's `epsilon2`, the tolerance used for circle tests.
pub const EPSILON2: f64 = 1e-12;

// ---------------------------------------------------------------------------
// Point accessors
// ---------------------------------------------------------------------------

/// A datum that carries a point, for the voronoi's default accessors.
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

impl Point for &[f64] {
    fn x(&self) -> f64 {
        self.first().copied().unwrap_or(f64::NAN)
    }
    fn y(&self) -> f64 {
        self.get(1).copied().unwrap_or(f64::NAN)
    }
}

// ---------------------------------------------------------------------------
// Accessor conversion
// ---------------------------------------------------------------------------

/// A value that can become a voronoi accessor.
pub trait IntoVoronoiAccessor<D> {
    /// Turn the value into a function that reads a number from a datum.
    fn into_voronoi_accessor(self) -> Arc<dyn Fn(&D) -> f64>;
}

impl<D, F> IntoVoronoiAccessor<D> for F
where
    F: Fn(&D) -> f64 + 'static,
{
    fn into_voronoi_accessor(self) -> Arc<dyn Fn(&D) -> f64> {
        Arc::new(self)
    }
}

impl<D> IntoVoronoiAccessor<D> for f64 {
    fn into_voronoi_accessor(self) -> Arc<dyn Fn(&D) -> f64> {
        let value = self;
        Arc::new(move |_| value)
    }
}

impl<D> IntoVoronoiAccessor<D> for i32 {
    fn into_voronoi_accessor(self) -> Arc<dyn Fn(&D) -> f64> {
        let value = self as f64;
        Arc::new(move |_| value)
    }
}

impl<D> IntoVoronoiAccessor<D> for i64 {
    fn into_voronoi_accessor(self) -> Arc<dyn Fn(&D) -> f64> {
        let value = self as f64;
        Arc::new(move |_| value)
    }
}

/// A value that can become a voronoi extent.
pub trait IntoVoronoiExtent {
    /// Turn the value into an extent, or `None`.
    fn into_voronoi_extent(self) -> Option<[[f64; 2]; 2]>;
}

impl IntoVoronoiExtent for Option<[[f64; 2]; 2]> {
    fn into_voronoi_extent(self) -> Option<[[f64; 2]; 2]> {
        self
    }
}

impl IntoVoronoiExtent for [[f64; 2]; 2] {
    fn into_voronoi_extent(self) -> Option<[[f64; 2]; 2]> {
        Some(self)
    }
}

/// A value that can become a voronoi size.
pub trait IntoVoronoiSize {
    /// Turn the value into a size, or `None`.
    fn into_voronoi_size(self) -> Option<[f64; 2]>;
}

impl IntoVoronoiSize for Option<[f64; 2]> {
    fn into_voronoi_size(self) -> Option<[f64; 2]> {
        self
    }
}

impl IntoVoronoiSize for [f64; 2] {
    fn into_voronoi_size(self) -> Option<[f64; 2]> {
        Some(self)
    }
}

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A site: the rounded point, its place in the input, and the datum.
#[derive(Clone, Debug)]
pub struct Site<D> {
    /// The rounded point.
    pub point: [f64; 2],
    /// The place of the datum in the input.
    pub index: usize,
    /// The datum.
    pub data: D,
}

/// A cell of the diagram.
#[derive(Clone, Debug)]
pub struct Cell<D> {
    /// The site the cell belongs to.
    pub site: Site<D>,
    /// Indices into the diagram's edges.
    pub halfedges: Vec<usize>,
}

/// An edge of the diagram.
#[derive(Clone, Debug)]
pub struct Edge<D> {
    /// The two endpoints of the edge.
    pub v: [Option<[f64; 2]>; 2],
    /// The site on the left of the edge.
    pub left: Option<Site<D>>,
    /// The site on the right of the edge.
    pub right: Option<Site<D>>,
}

impl<D> std::ops::Index<usize> for Edge<D> {
    type Output = Option<[f64; 2]>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.v[index]
    }
}

/// A polygon of the diagram.
#[derive(Clone, Debug)]
pub struct Polygon<D> {
    /// The vertices of the polygon.
    pub points: Vec<Option<[f64; 2]>>,
    /// The datum of the site.
    pub data: D,
}

impl<D> std::ops::Deref for Polygon<D> {
    type Target = [Option<[f64; 2]>];
    fn deref(&self) -> &Self::Target {
        &self.points
    }
}

/// A link between two sites.
#[derive(Clone, Debug)]
pub struct Link<D> {
    /// The source datum.
    pub source: D,
    /// The target datum.
    pub target: D,
}

/// A triangle of the diagram, made of three data.
pub type Triangle<D> = [D; 3];

/// The diagram the voronoi produces.
#[derive(Clone, Debug)]
pub struct Diagram<D> {
    /// The edges of the diagram, in the JavaScript's order.
    pub edges: Vec<Option<Edge<D>>>,
    /// The cells of the diagram, in the JavaScript's order.
    pub cells: Vec<Option<Cell<D>>>,
    found: usize,
    _marker: std::marker::PhantomData<D>,
}

impl<D: Clone> Diagram<D> {
    /// The JavaScript's `diagram.polygons`.
    pub fn polygons(&self) -> Vec<Option<Polygon<D>>> {
        self.cells
            .iter()
            .map(|cell_opt| {
                cell_opt.as_ref().map(|cell| {
                    let edges = &self.edges;
                    let points = cell
                        .halfedges
                        .iter()
                        .map(|&i| cell_halfedge_start(cell, edges[i].as_ref().unwrap()))
                        .collect();
                    Polygon {
                        points,
                        data: cell.site.data.clone(),
                    }
                })
            })
            .collect()
    }

    /// The JavaScript's `diagram.triangles`.
    pub fn triangles(&self) -> Vec<Triangle<D>> {
        let mut triangles = Vec::new();
        let edges = &self.edges;
        for (i, cell_opt) in self.cells.iter().enumerate() {
            let Some(cell) = cell_opt else {
                continue;
            };
            let m = cell.halfedges.len();
            if m == 0 {
                continue;
            }
            let site = &cell.site;
            let mut s1 = {
                let e = edges[cell.halfedges[m - 1]].as_ref().unwrap();
                if e.left.as_ref().map(|s| s.index) == Some(site.index) {
                    e.right.clone()
                } else {
                    e.left.clone()
                }
            };
            for j in 0..m {
                let s0 = s1.clone();
                let e = edges[cell.halfedges[j]].as_ref().unwrap();
                s1 = if e.left.as_ref().map(|s| s.index) == Some(site.index) {
                    e.right.clone()
                } else {
                    e.left.clone()
                };
                if let (Some(s0), Some(s1)) = (s0, s1.clone()) {
                    if i < s0.index && i < s1.index {
                        let area = (site.point[0] - s1.point[0]) * (s0.point[1] - site.point[1])
                            - (site.point[0] - s0.point[0]) * (s1.point[1] - site.point[1]);
                        if area < 0.0 {
                            triangles.push([site.data.clone(), s0.data.clone(), s1.data.clone()]);
                        }
                    }
                }
            }
        }
        triangles
    }

    /// The JavaScript's `diagram.links`.
    pub fn links(&self) -> Vec<Link<D>> {
        self.edges
            .iter()
            .filter_map(|e| e.as_ref())
            .filter(|e| e.right.is_some())
            .map(|e| Link {
                source: e.left.as_ref().unwrap().data.clone(),
                target: e.right.as_ref().unwrap().data.clone(),
            })
            .collect()
    }

    /// The JavaScript's `diagram.find`.
    pub fn find(&mut self, x: f64, y: f64, radius: Option<f64>) -> Option<Site<D>> {
        let n = self.cells.len();
        let mut i1 = self.found;
        loop {
            if i1 < n && self.cells[i1].is_some() {
                break;
            }
            i1 += 1;
            if i1 >= n {
                return None;
            }
        }
        let cell = self.cells[i1].as_ref().unwrap();
        let dx = x - cell.site.point[0];
        let dy = y - cell.site.point[1];
        let mut d2 = dx * dx + dy * dy;

        let mut i0;
        loop {
            i0 = i1;
            let cell = self.cells[i0].as_ref().unwrap().clone();
            let mut next: Option<usize> = None;
            for &ei in &cell.halfedges {
                let edge = match self.edges.get(ei).and_then(|e| e.as_ref()) {
                    Some(e) => e,
                    None => continue,
                };
                let mut v = edge.left.clone();
                let cond = v.as_ref().map(|s| s.index) == Some(cell.site.index) || v.is_none();
                if cond {
                    v = edge.right.clone();
                    if v.is_none() {
                        continue;
                    }
                }
                if let Some(vsite) = v {
                    let vx = x - vsite.point[0];
                    let vy = y - vsite.point[1];
                    let v2 = vx * vx + vy * vy;
                    if v2 < d2 {
                        d2 = v2;
                        next = Some(vsite.index);
                    }
                }
            }
            match next {
                Some(idx) => i1 = idx,
                None => break,
            }
        }

        self.found = i0;
        if radius.map(|r| d2 <= r * r).unwrap_or(true) {
            Some(self.cells[i0].as_ref().unwrap().site.clone())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Voronoi generator
// ---------------------------------------------------------------------------

/// The voronoi generator.
pub struct Voronoi<D> {
    x: Arc<dyn Fn(&D) -> f64>,
    y: Arc<dyn Fn(&D) -> f64>,
    extent: Option<[[f64; 2]; 2]>,
    _marker: std::marker::PhantomData<D>,
}

impl<D: Point> Voronoi<D> {
    /// The JavaScript's `d3.voronoi()`: a generator with the default
    /// accessors.
    pub fn new() -> Self {
        Self::with_accessors(|d: &D| d.x(), |d: &D| d.y())
    }
}

impl<D> Default for Voronoi<D>
where
    D: Point,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Voronoi<D> {
    /// Build a generator from two accessors.
    pub fn with_accessors<X, Y>(x: X, y: Y) -> Self
    where
        X: IntoVoronoiAccessor<D>,
        Y: IntoVoronoiAccessor<D>,
    {
        Self {
            x: x.into_voronoi_accessor(),
            y: y.into_voronoi_accessor(),
            extent: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// The JavaScript's `voronoi.x` setter, consuming form.
    pub fn x<X: IntoVoronoiAccessor<D>>(mut self, x: X) -> Self {
        self.x = x.into_voronoi_accessor();
        self
    }

    /// The JavaScript's `voronoi.x` setter, mutating form.
    pub fn set_x<X: IntoVoronoiAccessor<D>>(&mut self, x: X) -> &mut Self {
        self.x = x.into_voronoi_accessor();
        self
    }

    /// The JavaScript's `voronoi.x` getter.
    pub fn get_x(&self) -> &Arc<dyn Fn(&D) -> f64> {
        &self.x
    }

    /// Alias for the getter.
    pub fn x_get(&self) -> &Arc<dyn Fn(&D) -> f64> {
        &self.x
    }

    /// The JavaScript's `voronoi.y` setter, consuming form.
    pub fn y<X: IntoVoronoiAccessor<D>>(mut self, y: X) -> Self {
        self.y = y.into_voronoi_accessor();
        self
    }

    /// The JavaScript's `voronoi.y` setter, mutating form.
    pub fn set_y<X: IntoVoronoiAccessor<D>>(&mut self, y: X) -> &mut Self {
        self.y = y.into_voronoi_accessor();
        self
    }

    /// The JavaScript's `voronoi.y` getter.
    pub fn get_y(&self) -> &Arc<dyn Fn(&D) -> f64> {
        &self.y
    }

    /// Alias for the getter.
    pub fn y_get(&self) -> &Arc<dyn Fn(&D) -> f64> {
        &self.y
    }

    /// The JavaScript's `voronoi.extent` setter, consuming form.
    pub fn extent<E: IntoVoronoiExtent>(mut self, extent: E) -> Self {
        self.extent = extent.into_voronoi_extent().map(normalize_extent_value);
        self
    }

    /// The JavaScript's `voronoi.extent` setter, mutating form.
    pub fn set_extent<E: IntoVoronoiExtent>(&mut self, extent: E) -> &mut Self {
        self.extent = extent.into_voronoi_extent().map(normalize_extent_value);
        self
    }

    /// The JavaScript's `voronoi.extent` getter.
    pub fn get_extent(&self) -> Option<[[f64; 2]; 2]> {
        self.extent
    }

    /// Alias for the getter.
    pub fn extent_get(&self) -> Option<[[f64; 2]; 2]> {
        self.extent
    }

    /// The JavaScript's `voronoi.size` setter, consuming form.
    pub fn size<S: IntoVoronoiSize>(mut self, size: S) -> Self {
        self.extent = size.into_voronoi_size().map(|s| [[0.0, 0.0], [s[0], s[1]]]);
        self
    }

    /// The JavaScript's `voronoi.size` setter, mutating form.
    pub fn set_size<S: IntoVoronoiSize>(&mut self, size: S) -> &mut Self {
        self.extent = size.into_voronoi_size().map(|s| [[0.0, 0.0], [s[0], s[1]]]);
        self
    }

    /// The JavaScript's `voronoi.size` getter.
    pub fn get_size(&self) -> Option<[f64; 2]> {
        self.extent.map(|e| [e[1][0] - e[0][0], e[1][1] - e[0][1]])
    }

    /// Alias for the getter.
    pub fn size_get(&self) -> Option<[f64; 2]> {
        self.get_size()
    }

    /// The JavaScript's `voronoi(data)`.
    pub fn call(&self, data: &[D]) -> Diagram<D>
    where
        D: Clone,
    {
        let mut sites = Vec::with_capacity(data.len());
        for (i, d) in data.iter().enumerate() {
            let x = js::round((self.x)(d) / EPSILON) * EPSILON;
            let y = js::round((self.y)(d) / EPSILON) * EPSILON;
            sites.push(Site {
                point: [x, y],
                index: i,
                data: d.clone(),
            });
        }
        Diagram::new(sites, self.extent)
    }

    /// Alias for `call`.
    pub fn apply(&self, data: &[D]) -> Diagram<D>
    where
        D: Clone,
    {
        self.call(data)
    }

    /// Convenience: `voronoi.polygons(data)`.
    pub fn polygons(&self, data: &[D]) -> Vec<Option<Polygon<D>>>
    where
        D: Clone,
    {
        self.call(data).polygons()
    }

    /// Convenience: `voronoi.triangles(data)`.
    pub fn triangles(&self, data: &[D]) -> Vec<Triangle<D>>
    where
        D: Clone,
    {
        self.call(data).triangles()
    }

    /// Convenience: `voronoi.links(data)`.
    pub fn links(&self, data: &[D]) -> Vec<Link<D>>
    where
        D: Clone,
    {
        self.call(data).links()
    }
}

fn normalize_extent_value(e: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [[e[0][0], e[0][1]], [e[1][0], e[1][1]]]
}

/// The JavaScript's `d3.voronoi`: a generator with the default accessors.
pub fn voronoi<D: Point>() -> Voronoi<D> {
    Voronoi::new()
}

// ---------------------------------------------------------------------------
// Diagram construction
// ---------------------------------------------------------------------------

impl<D: Clone> Diagram<D> {
    pub(crate) fn new(sites_input: Vec<Site<D>>, extent: Option<[[f64; 2]; 2]>) -> Self {
        // sort lexicographic
        let mut sites = sites_input;
        sites.sort_by(|a, b| {
            let cy = b.point[1]
                .partial_cmp(&a.point[1])
                .unwrap_or(Ordering::Equal);
            if cy != Ordering::Equal {
                return cy;
            }
            b.point[0]
                .partial_cmp(&a.point[0])
                .unwrap_or(Ordering::Equal)
        });

        let mut builder = Builder {
            edges: Vec::new(),
            cells: vec![None; sites.len()],
            beach_head: None,
            circles: Vec::new(),
            first_circle: None,
        };

        let mut site = sites.pop();
        let mut x: Option<f64> = None;
        let mut y: Option<f64> = None;

        loop {
            let circle = builder.first_circle.clone();
            let take_site = if let Some(s) = &site {
                match &circle {
                    None => true,
                    Some(c) => {
                        let cb = c.borrow();
                        s.point[1] < cb.y || (s.point[1] == cb.y && s.point[0] < cb.x)
                    }
                }
            } else {
                false
            };
            if take_site {
                let s = site.take().unwrap();
                if x != Some(s.point[0]) || y != Some(s.point[1]) {
                    add_beach(&mut builder, s.clone());
                    x = Some(s.point[0]);
                    y = Some(s.point[1]);
                }
                site = sites.pop();
            } else if circle.is_some() {
                let c = circle.unwrap();
                let arc = c.borrow().arc.clone().unwrap();
                remove_beach(&mut builder, arc);
            } else {
                break;
            }
        }

        sort_cell_halfedges(&mut builder.cells, &builder.edges);

        if let Some(extent) = extent {
            let x0 = extent[0][0];
            let y0 = extent[0][1];
            let x1 = extent[1][0];
            let y1 = extent[1][1];
            clip_edges(&mut builder, x0, y0, x1, y1);
            clip_cells(&mut builder, x0, y0, x1, y1);
        }

        Diagram {
            edges: builder.edges,
            cells: builder.cells,
            found: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

struct Builder<D> {
    edges: Vec<Option<Edge<D>>>,
    cells: Vec<Option<Cell<D>>>,
    beach_head: Option<Rc<RefCell<Beach<D>>>>,
    circles: Vec<Rc<RefCell<Circle<D>>>>,
    first_circle: Option<Rc<RefCell<Circle<D>>>>,
}

#[derive(Clone)]
struct Beach<D> {
    u: Option<Rc<RefCell<Beach<D>>>>,
    c: bool,
    l: Option<Rc<RefCell<Beach<D>>>>,
    r: Option<Rc<RefCell<Beach<D>>>>,
    p: Option<Rc<RefCell<Beach<D>>>>,
    n: Option<Rc<RefCell<Beach<D>>>>,
    edge: Option<usize>,
    site: Site<D>,
    circle: Option<Rc<RefCell<Circle<D>>>>,
}

#[derive(Clone)]
struct Circle<D> {
    u: Option<Rc<RefCell<Circle<D>>>>,
    c: bool,
    l: Option<Rc<RefCell<Circle<D>>>>,
    r: Option<Rc<RefCell<Circle<D>>>>,
    p: Option<Rc<RefCell<Circle<D>>>>,
    n: Option<Rc<RefCell<Circle<D>>>>,
    x: f64,
    y: f64,
    arc: Option<Rc<RefCell<Beach<D>>>>,
    cy: f64,
}

// ---------------------------------------------------------------------------
// Beach/circle helpers
// ---------------------------------------------------------------------------

fn create_cell<D: Clone>(builder: &mut Builder<D>, site: Site<D>) {
    builder.cells[site.index] = Some(Cell {
        site: site.clone(),
        halfedges: Vec::new(),
    });
}

fn create_edge<D: Clone>(
    builder: &mut Builder<D>,
    left: Site<D>,
    right: Site<D>,
    v0: Option<[f64; 2]>,
    v1: Option<[f64; 2]>,
) -> usize {
    let edge = Edge {
        v: [None, None],
        left: Some(left.clone()),
        right: Some(right.clone()),
    };
    let index = builder.edges.len();
    builder.edges.push(Some(edge));
    if let Some(v) = v0 {
        set_edge_end_idx(builder, index, left.clone(), right.clone(), v);
    }
    if let Some(v) = v1 {
        set_edge_end_idx(builder, index, right.clone(), left.clone(), v);
    }
    if let Some(cell) = builder.cells[left.index].as_mut() {
        cell.halfedges.push(index);
    }
    if let Some(cell) = builder.cells[right.index].as_mut() {
        cell.halfedges.push(index);
    }
    index
}

fn create_border_edge<D: Clone>(
    builder: &mut Builder<D>,
    left: Site<D>,
    v0: [f64; 2],
    v1: Option<[f64; 2]>,
) -> usize {
    let edge = Edge {
        v: [Some(v0), v1],
        left: Some(left),
        right: None,
    };
    let index = builder.edges.len();
    builder.edges.push(Some(edge));
    index
}

fn set_edge_end_idx<D: Clone>(
    builder: &mut Builder<D>,
    ei: usize,
    left: Site<D>,
    right: Site<D>,
    vertex: [f64; 2],
) {
    let edge = builder.edges[ei].as_mut().unwrap();
    if edge.v[0].is_none() && edge.v[1].is_none() {
        edge.v[0] = Some(vertex);
        edge.left = Some(left);
        edge.right = Some(right);
    } else if edge.left.as_ref().map(|s| s.index) == Some(right.index) {
        edge.v[1] = Some(vertex);
    } else {
        edge.v[0] = Some(vertex);
    }
}

fn detach_circle_beach<D: Clone>(builder: &mut Builder<D>, beach: &Rc<RefCell<Beach<D>>>) {
    let circle = beach.borrow_mut().circle.take();
    if let Some(c) = circle {
        if c.borrow().p.is_none() {
            builder.first_circle = c.borrow().n.clone();
        }
        // reset
        let mut cc = c.borrow_mut();
        cc.u = None;
        cc.c = false;
        cc.l = None;
        cc.r = None;
        cc.p = None;
        cc.n = None;
        cc.arc = None;
    }
}

fn detach_beach<D: Clone>(builder: &mut Builder<D>, beach: &Rc<RefCell<Beach<D>>>) {
    detach_circle_beach(builder, beach);
    // linked list removal
    if let Some(p) = beach.borrow().p.clone() {
        p.borrow_mut().n = beach.borrow().n.clone();
    } else {
        builder.beach_head = beach.borrow().n.clone();
    }
    if let Some(n) = beach.borrow().n.clone() {
        n.borrow_mut().p = beach.borrow().p.clone();
    }
    let mut bb = beach.borrow_mut();
    bb.u = None;
    bb.c = false;
    bb.l = None;
    bb.r = None;
    bb.p = None;
    bb.n = None;
}

fn attach_circle_beach<D: Clone>(builder: &mut Builder<D>, arc: &Rc<RefCell<Beach<D>>>) {
    let l_arc = arc.borrow().p.clone();
    let r_arc = arc.borrow().n.clone();
    let (Some(l_arc), Some(r_arc)) = (l_arc, r_arc) else {
        return;
    };
    let l_site = l_arc.borrow().site.clone();
    let c_site = arc.borrow().site.clone();
    let r_site = r_arc.borrow().site.clone();
    if l_site.index == c_site.index {
        return;
    }
    let bx = c_site.point[0];
    let by = c_site.point[1];
    let ax = l_site.point[0] - bx;
    let ay = l_site.point[1] - by;
    let cx = r_site.point[0] - bx;
    let cy = r_site.point[1] - by;
    let d = 2.0 * (ax * cy - ay * cx);
    if d >= -EPSILON2 {
        return;
    }
    let ha = ax * ax + ay * ay;
    let hc = cx * cx + cy * cy;
    let x = (cy * ha - ay * hc) / d;
    let y = (ax * hc - cx * ha) / d;
    let circle = Rc::new(RefCell::new(Circle {
        u: None,
        c: false,
        l: None,
        r: None,
        p: None,
        n: None,
        x: x + bx,
        y: (y + by) + (x * x + y * y).sqrt(),
        arc: Some(arc.clone()),
        cy: y + by,
    }));
    arc.borrow_mut().circle = Some(circle.clone());

    builder.circles.push(circle.clone());
    builder.circles.sort_by(|a, b| {
        let ab = a.borrow();
        let bb = b.borrow();
        ab.y.partial_cmp(&bb.y)
            .unwrap_or(Ordering::Equal)
            .then(ab.x.partial_cmp(&bb.x).unwrap_or(Ordering::Equal))
    });
    builder.first_circle = builder.circles.first().cloned();
}

fn insert_beach_list<D: Clone>(
    builder: &mut Builder<D>,
    after: Option<Rc<RefCell<Beach<D>>>>,
    node: Rc<RefCell<Beach<D>>>,
) {
    // reset linked list pointers
    node.borrow_mut().p = None;
    node.borrow_mut().n = None;
    if let Some(after_node) = after {
        node.borrow_mut().p = Some(after_node.clone());
        node.borrow_mut().n = after_node.borrow().n.clone();
        if let Some(nn) = node.borrow().n.clone() {
            nn.borrow_mut().p = Some(node.clone());
        }
        after_node.borrow_mut().n = Some(node.clone());
    } else if let Some(head) = builder.beach_head.clone() {
        node.borrow_mut().n = Some(head.clone());
        head.borrow_mut().p = Some(node.clone());
        builder.beach_head = Some(node);
    } else {
        builder.beach_head = Some(node);
    }
}

fn add_beach<D: Clone>(mut builder: &mut Builder<D>, site: Site<D>) {
    create_cell(builder, site.clone());

    let x = site.point[0];
    let directrix = site.point[1];

    let mut l_arc: Option<Rc<RefCell<Beach<D>>>> = None;
    let mut r_arc: Option<Rc<RefCell<Beach<D>>>> = None;

    let mut cur = builder.beach_head.clone();
    while let Some(n) = cur.clone() {
        let dxl = left_breakpoint_beach(&n, directrix) - x;
        if dxl > EPSILON {
            cur = n.borrow().p.clone();
            continue;
        }
        let dxr = x - right_breakpoint_beach(&n, directrix);
        if dxr > EPSILON {
            let nxt = n.borrow().n.clone();
            if nxt.is_none() {
                l_arc = Some(n.clone());
                break;
            }
            cur = nxt;
            continue;
        } else {
            if dxl > -EPSILON {
                l_arc = n.borrow().p.clone();
                r_arc = Some(n.clone());
            } else if dxr > -EPSILON {
                l_arc = Some(n.clone());
                r_arc = n.borrow().n.clone();
            } else {
                l_arc = Some(n.clone());
                r_arc = Some(n.clone());
            }
            break;
        }
    }

    let new_arc = Rc::new(RefCell::new(Beach {
        u: None,
        c: false,
        l: None,
        r: None,
        p: None,
        n: None,
        edge: None,
        site: site.clone(),
        circle: None,
    }));
    insert_beach_list(builder, l_arc.clone(), new_arc.clone());

    if l_arc.is_none() && r_arc.is_none() {
        return;
    }

    if l_arc.is_some()
        && r_arc.is_some()
        && Rc::ptr_eq(l_arc.as_ref().unwrap(), r_arc.as_ref().unwrap())
    {
        if let Some(l) = l_arc.clone() {
            detach_circle_beach(builder, &l);
            let r_new = Rc::new(RefCell::new(Beach {
                u: None,
                c: false,
                l: None,
                r: None,
                p: None,
                n: None,
                edge: None,
                site: l.borrow().site.clone(),
                circle: None,
            }));
            insert_beach_list(builder, Some(new_arc.clone()), r_new.clone());
            let l_site = l.borrow().site.clone();
            let new_site = new_arc.borrow().site.clone();
            let ei = create_edge(builder, l_site, new_site, None, None);
            new_arc.borrow_mut().edge = Some(ei);
            r_new.borrow_mut().edge = Some(ei);
            attach_circle_beach(builder, &l);
            attach_circle_beach(builder, &r_new);
        }
        return;
    }

    if r_arc.is_none() {
        if let Some(l) = l_arc.clone() {
            let ei = create_edge(
                builder,
                l.borrow().site.clone(),
                new_arc.borrow().site.clone(),
                None,
                None,
            );
            new_arc.borrow_mut().edge = Some(ei);
        }
        return;
    }

    // l_arc !== r_arc
    if let (Some(l), Some(r)) = (l_arc.clone(), r_arc.clone()) {
        detach_circle_beach(builder, &l);
        detach_circle_beach(builder, &r);

        let l_site = l.borrow().site.clone();
        let ax = l_site.point[0];
        let ay = l_site.point[1];
        let bx = site.point[0] - ax;
        let by = site.point[1] - ay;
        let r_site = r.borrow().site.clone();
        let cx = r_site.point[0] - ax;
        let cy = r_site.point[1] - ay;
        let d = 2.0 * (bx * cy - by * cx);
        let hb = bx * bx + by * by;
        let hc = cx * cx + cy * cy;
        let vx = (cy * hb - by * hc) / d + ax;
        let vy = (bx * hc - cx * hb) / d + ay;
        let vertex = [vx, vy];

        if let Some(ei) = r.borrow().edge {
            set_edge_end_idx(builder, ei, l_site.clone(), r_site.clone(), vertex);
        }
        let ei1 = create_edge(builder, l_site.clone(), site.clone(), None, Some(vertex));
        new_arc.borrow_mut().edge = Some(ei1);
        let ei2 = create_edge(builder, site.clone(), r_site.clone(), None, Some(vertex));
        r.borrow_mut().edge = Some(ei2);

        attach_circle_beach(builder, &l);
        attach_circle_beach(builder, &r);
    }
}

fn remove_beach<D: Clone>(builder: &mut Builder<D>, beach: Rc<RefCell<Beach<D>>>) {
    let circle_rc = beach.borrow().circle.clone().unwrap();
    let cx = circle_rc.borrow().x;
    let cy = circle_rc.borrow().cy;
    let vertex = [cx, cy];

    let previous = beach.borrow().p.clone();
    let next = beach.borrow().n.clone();

    detach_beach(builder, &beach);

    let mut disappearing: Vec<Rc<RefCell<Beach<D>>>> = vec![beach.clone()];
    let mut l_arc = previous.clone();
    while l_arc
        .as_ref()
        .map(|a| a.borrow().circle.is_some())
        .unwrap_or(false)
    {
        let c = l_arc.as_ref().unwrap().borrow().circle.clone().unwrap();
        let cc = c.borrow();
        if (cx - cc.x).abs() >= EPSILON || (cy - cc.cy).abs() >= EPSILON {
            break;
        }
        drop(cc);
        let prev = l_arc.as_ref().unwrap().borrow().p.clone();
        disappearing.insert(0, l_arc.as_ref().unwrap().clone());
        detach_beach(builder, &l_arc.as_ref().unwrap().clone());
        l_arc = prev;
    }
    if let Some(l) = l_arc.clone() {
        disappearing.insert(0, l.clone());
        detach_circle_beach(builder, &l);
    }

    let mut r_arc = next.clone();
    while r_arc
        .as_ref()
        .map(|a| a.borrow().circle.is_some())
        .unwrap_or(false)
    {
        let c = r_arc.as_ref().unwrap().borrow().circle.clone().unwrap();
        let cc = c.borrow();
        if (cx - cc.x).abs() >= EPSILON || (cy - cc.cy).abs() >= EPSILON {
            break;
        }
        drop(cc);
        let nxt = r_arc.as_ref().unwrap().borrow().n.clone();
        disappearing.push(r_arc.as_ref().unwrap().clone());
        detach_beach(builder, &r_arc.as_ref().unwrap().clone());
        r_arc = nxt;
    }
    if let Some(r) = r_arc.clone() {
        disappearing.push(r.clone());
        detach_circle_beach(builder, &r);
    }

    let n_arcs = disappearing.len();
    for i in 1..n_arcs {
        let r = disappearing[i].clone();
        let l = disappearing[i - 1].clone();
        let left_site = l.borrow().site.clone();
        let right_site = r.borrow().site.clone();
        let ei_opt = r.borrow().edge;
        if let Some(ei) = ei_opt {
            set_edge_end_idx(builder, ei, left_site, right_site, vertex);
        }
    }
    let l0 = disappearing[0].clone();
    let rlast = disappearing[n_arcs - 1].clone();
    let l_site = l0.borrow().site.clone();
    let r_site = rlast.borrow().site.clone();
    let ei = create_edge(builder, l_site, r_site, None, Some(vertex));
    rlast.borrow_mut().edge = Some(ei);

    attach_circle_beach(builder, &l0);
    attach_circle_beach(builder, &rlast);
}

// ---------------------------------------------------------------------------
// Breakpoints
// ---------------------------------------------------------------------------

fn left_breakpoint_beach<D: Clone>(arc: &Rc<RefCell<Beach<D>>>, directrix: f64) -> f64 {
    let site = arc.borrow().site.clone();
    let rfocx = site.point[0];
    let rfocy = site.point[1];
    let pby2 = rfocy - directrix;
    if pby2 == 0.0 {
        return rfocx;
    }
    let l_arc = arc.borrow().p.clone();
    let Some(l_arc) = l_arc else {
        return f64::NEG_INFINITY;
    };
    let lsite = l_arc.borrow().site.clone();
    let lfocx = lsite.point[0];
    let lfocy = lsite.point[1];
    let plby2 = lfocy - directrix;
    if plby2 == 0.0 {
        return lfocx;
    }
    let hl = lfocx - rfocx;
    let aby2 = 1.0 / pby2 - 1.0 / plby2;
    let b = hl / plby2;
    if aby2 != 0.0 {
        (-b + (b * b
            - 2.0 * aby2 * (hl * hl / (-2.0 * plby2) - lfocy + plby2 / 2.0 + rfocy - pby2 / 2.0))
            .sqrt())
            / aby2
            + rfocx
    } else {
        (rfocx + lfocx) / 2.0
    }
}

fn right_breakpoint_beach<D: Clone>(arc: &Rc<RefCell<Beach<D>>>, directrix: f64) -> f64 {
    if let Some(r) = arc.borrow().n.clone() {
        left_breakpoint_beach(&r, directrix)
    } else {
        let site = arc.borrow().site.clone();
        if site.point[1] == directrix {
            site.point[0]
        } else {
            f64::INFINITY
        }
    }
}

// ---------------------------------------------------------------------------
// Sorting / clipping
// ---------------------------------------------------------------------------

fn sort_cell_halfedges<D: Clone>(cells: &mut Vec<Option<Cell<D>>>, edges: &[Option<Edge<D>>]) {
    let n = cells.len();
    for i in 0..n {
        let Some(cell) = cells[i].as_mut() else {
            continue;
        };
        let m = cell.halfedges.len();
        if m == 0 {
            continue;
        }
        let mut index: Vec<usize> = (0..m).collect();
        let array: Vec<f64> = (0..m)
            .map(|j| cell_halfedge_angle(cell, edges[cell.halfedges[j]].as_ref().unwrap()))
            .collect();
        index.sort_by(|&a, &b| array[b].partial_cmp(&array[a]).unwrap_or(Ordering::Equal));
        let mut reordered = vec![0usize; m];
        for j in 0..m {
            reordered[j] = cell.halfedges[index[j]];
        }
        cell.halfedges = reordered;
        let _ = array;
    }
}

fn cell_halfedge_angle<D: Clone>(cell: &Cell<D>, edge: &Edge<D>) -> f64 {
    let site = &cell.site;
    let va_site = edge.left.clone();
    let vb_site = edge.right.clone();
    if vb_site.as_ref().map(|s| s.index) == Some(site.index) {
        // va = site, vb = va(old)
        // need old va
        let old_va = va_site.clone();
        // vb becomes old_va, va becomes site
        if let Some(vb) = old_va {
            let va_pt = site.point;
            let vb_pt = vb.point;
            return (vb_pt[1] - va_pt[1]).atan2(vb_pt[0] - va_pt[0]);
        }
    }
    if vb_site.is_some() {
        let va_pt = va_site.as_ref().unwrap().point;
        let vb_pt = vb_site.as_ref().unwrap().point;
        return (vb_pt[1] - va_pt[1]).atan2(vb_pt[0] - va_pt[0]);
    }
    // border edge
    let va_pt = if va_site.as_ref().map(|s| s.index) == Some(site.index) {
        edge.v[1].unwrap()
    } else {
        edge.v[0].unwrap()
    };
    let vb_pt = if va_site.as_ref().map(|s| s.index) == Some(site.index) {
        edge.v[0].unwrap()
    } else {
        edge.v[1].unwrap()
    };
    (va_pt[0] - vb_pt[0]).atan2(vb_pt[1] - va_pt[1])
}

fn cell_halfedge_start<D: Clone>(cell: &Cell<D>, edge: &Edge<D>) -> Option<[f64; 2]> {
    let idx = if edge.left.as_ref().map(|s| s.index) != Some(cell.site.index) {
        1
    } else {
        0
    };
    edge.v[idx]
}

fn cell_halfedge_end<D: Clone>(cell: &Cell<D>, edge: &Edge<D>) -> Option<[f64; 2]> {
    let idx = if edge.left.as_ref().map(|s| s.index) == Some(cell.site.index) {
        1
    } else {
        0
    };
    edge.v[idx]
}

fn connect_edge<D: Clone>(edge: &mut Edge<D>, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
    if edge.v[1].is_some() {
        return true;
    }
    let v0 = edge.v[0];
    let left = edge.left.clone().unwrap();
    let right = edge.right.clone().unwrap();
    let lx = left.point[0];
    let ly = left.point[1];
    let rx = right.point[0];
    let ry = right.point[1];
    let fx = (lx + rx) / 2.0;
    let fy = (ly + ry) / 2.0;

    let mut v0 = v0;
    let v1: Option<[f64; 2]>;

    if ry == ly {
        if fx < x0 || fx >= x1 {
            return false;
        }
        if lx > rx {
            if v0.is_none() {
                v0 = Some([fx, y0]);
            } else if v0.unwrap()[1] >= y1 {
                return false;
            }
            v1 = Some([fx, y1]);
        } else {
            if v0.is_none() {
                v0 = Some([fx, y1]);
            } else if v0.unwrap()[1] < y0 {
                return false;
            }
            v1 = Some([fx, y0]);
        }
    } else {
        let fm = (lx - rx) / (ry - ly);
        let fb = fy - fm * fx;
        if fm < -1.0 || fm > 1.0 {
            if lx > rx {
                if v0.is_none() {
                    v0 = Some([(y0 - fb) / fm, y0]);
                } else if v0.unwrap()[1] >= y1 {
                    return false;
                }
                v1 = Some([(y1 - fb) / fm, y1]);
            } else {
                if v0.is_none() {
                    v0 = Some([(y1 - fb) / fm, y1]);
                } else if v0.unwrap()[1] < y0 {
                    return false;
                }
                v1 = Some([(y0 - fb) / fm, y0]);
            }
        } else if ly < ry {
            if v0.is_none() {
                v0 = Some([x0, fm * x0 + fb]);
            } else if v0.unwrap()[0] >= x1 {
                return false;
            }
            v1 = Some([x1, fm * x1 + fb]);
        } else {
            if v0.is_none() {
                v0 = Some([x1, fm * x1 + fb]);
            } else if v0.unwrap()[0] < x0 {
                return false;
            }
            v1 = Some([x0, fm * x0 + fb]);
        }
    }

    edge.v[0] = v0;
    edge.v[1] = v1;
    true
}

fn clip_edge<D: Clone>(edge: &mut Edge<D>, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
    let a = edge.v[0].unwrap();
    let b = edge.v[1].unwrap();
    let ax = a[0];
    let ay = a[1];
    let bx = b[0];
    let by = b[1];
    let mut t0 = 0.0;
    let mut t1 = 1.0;
    let dx = bx - ax;
    let dy = by - ay;

    // x0
    {
        let mut r = x0 - ax;
        if dx == 0.0 {
            if r > 0.0 {
                return false;
            }
        } else {
            r /= dx;
            if dx < 0.0 {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            } else {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            }
        }
    }
    // x1
    {
        let mut r = x1 - ax;
        if dx == 0.0 {
            if r < 0.0 {
                return false;
            }
        } else {
            r /= dx;
            if dx < 0.0 {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }
    // y0
    {
        let mut r = y0 - ay;
        if dy == 0.0 {
            if r > 0.0 {
                return false;
            }
        } else {
            r /= dy;
            if dy < 0.0 {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            } else {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            }
        }
    }
    // y1
    {
        let mut r = y1 - ay;
        if dy == 0.0 {
            if r < 0.0 {
                return false;
            }
        } else {
            r /= dy;
            if dy < 0.0 {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }

    if !(t0 > 0.0) && !(t1 < 1.0) {
        // keep
    } else {
        if t0 > 0.0 {
            edge.v[0] = Some([ax + t0 * dx, ay + t0 * dy]);
        }
        if t1 < 1.0 {
            edge.v[1] = Some([ax + t1 * dx, ay + t1 * dy]);
        }
    }
    true
}

fn clip_edges<D: Clone>(builder: &mut Builder<D>, x0: f64, y0: f64, x1: f64, y1: f64) {
    let mut i = builder.edges.len();
    while i > 0 {
        i -= 1;
        let Some(edge) = builder.edges[i].as_mut() else {
            continue;
        };
        // connect
        let connected = connect_edge(edge, x0, y0, x1, y1);
        if !connected {
            builder.edges[i] = None;
            continue;
        }
        let clipped = clip_edge(edge, x0, y0, x1, y1);
        if !clipped {
            builder.edges[i] = None;
            continue;
        }
        let a = edge.v[0].unwrap();
        let b = edge.v[1].unwrap();
        if !((a[0] - b[0]).abs() > EPSILON || (a[1] - b[1]).abs() > EPSILON) {
            builder.edges[i] = None;
        }
    }
}

fn clip_cells<D: Clone>(builder: &mut Builder<D>, x0: f64, y0: f64, x1: f64, y1: f64) {
    let n_cells = builder.cells.len();
    let mut cover = true;

    for i_cell in 0..n_cells {
        let Some(_) = builder.cells[i_cell].as_ref() else {
            continue;
        };
        let site = builder.cells[i_cell].as_ref().unwrap().site.clone();

        // Remove dangling clipped edges.
        {
            let cell = builder.cells[i_cell].as_mut().unwrap();
            let mut i = cell.halfedges.len();
            while i > 0 {
                i -= 1;
                let ei = cell.halfedges[i];
                if builder.edges.get(ei).and_then(|e| e.as_ref()).is_none() {
                    cell.halfedges.remove(i);
                }
            }
        }

        // Insert border edges.
        let mut i_halfedge = 0usize;
        loop {
            let n_halfedges = match builder.cells[i_cell].as_ref() {
                Some(c) => c.halfedges.len(),
                None => 0,
            };
            if i_halfedge >= n_halfedges {
                break;
            }
            // end from current
            let cell = builder.cells[i_cell].as_ref().unwrap();
            let ei_cur = cell.halfedges[i_halfedge];
            let edge_cur = builder.edges[ei_cur].as_ref().unwrap();
            let end = cell_halfedge_end(cell, edge_cur).unwrap();
            // start from next
            let next_idx = (i_halfedge + 1) % n_halfedges;
            let ei_next = cell.halfedges[next_idx];
            let edge_next = builder.edges[ei_next].as_ref().unwrap();
            let start = cell_halfedge_start(cell, edge_next).unwrap();

            if (end[0] - start[0]).abs() > EPSILON || (end[1] - start[1]).abs() > EPSILON {
                // compute border point
                let v1opt = if (end[0] - x0).abs() < EPSILON && y1 - end[1] > EPSILON {
                    Some(if (start[0] - x0).abs() < EPSILON {
                        [x0, start[1]]
                    } else {
                        [x0, y1]
                    })
                } else if (end[1] - y1).abs() < EPSILON && x1 - end[0] > EPSILON {
                    Some(if (start[1] - y1).abs() < EPSILON {
                        [start[0], y1]
                    } else {
                        [x1, y1]
                    })
                } else if (end[0] - x1).abs() < EPSILON && end[1] - y0 > EPSILON {
                    Some(if (start[0] - x1).abs() < EPSILON {
                        [x1, start[1]]
                    } else {
                        [x1, y0]
                    })
                } else if (end[1] - y0).abs() < EPSILON && end[0] - x0 > EPSILON {
                    Some(if (start[1] - y0).abs() < EPSILON {
                        [start[0], y0]
                    } else {
                        [x0, y0]
                    })
                } else {
                    None
                };

                let border_ei = create_border_edge(builder, site.clone(), end, v1opt);
                let cell = builder.cells[i_cell].as_mut().unwrap();
                cell.halfedges.insert(i_halfedge + 1, border_ei);
            }
            i_halfedge += 1;
        }

        let has = builder.cells[i_cell].as_ref().unwrap().halfedges.len() > 0;
        if has {
            cover = false;
        }
    }

    if cover {
        // find closest cell to (x0, y0)
        let mut dc = f64::INFINITY;
        let mut cover_idx: Option<usize> = None;
        for i_cell in 0..n_cells {
            if let Some(cell) = builder.cells[i_cell].as_ref() {
                let site = &cell.site;
                let dx = site.point[0] - x0;
                let dy = site.point[1] - y0;
                let d2 = dx * dx + dy * dy;
                if d2 < dc {
                    dc = d2;
                    cover_idx = Some(i_cell);
                }
            }
        }
        if let Some(ci) = cover_idx {
            let site = builder.cells[ci].as_ref().unwrap().site.clone();
            let v00 = [x0, y0];
            let v01 = [x0, y1];
            let v11 = [x1, y1];
            let v10 = [x1, y0];
            let e1 = create_border_edge(builder, site.clone(), v00, Some(v01));
            let e2 = create_border_edge(builder, site.clone(), v01, Some(v11));
            let e3 = create_border_edge(builder, site.clone(), v11, Some(v10));
            let e4 = create_border_edge(builder, site.clone(), v10, Some(v00));
            let cell = builder.cells[ci].as_mut().unwrap();
            cell.halfedges.push(e1);
            cell.halfedges.push(e2);
            cell.halfedges.push(e3);
            cell.halfedges.push(e4);
        }
    }

    // delete empty cells
    for i in 0..n_cells {
        if let Some(cell) = &builder.cells[i] {
            if cell.halfedges.is_empty() {
                builder.cells[i] = None;
            }
        }
    }
}
