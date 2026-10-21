//! The port of d3-shape, checked against d3's own JavaScript.
//!
//! The fixture this test reads was produced by running d3-shape 1.3.7
//! under Node — the same code the browser runs — and writing down what it
//! produced.

use std::fs;

use d3::path::Path;
use d3::shape::{
    curve_basis, curve_basis_closed, curve_basis_open, curve_bundle, curve_cardinal,
    curve_cardinal_closed, curve_cardinal_open, curve_catmull_rom, curve_catmull_rom_closed,
    curve_catmull_rom_open, curve_linear_closed, curve_monotone_x, curve_monotone_y, curve_natural,
    curve_step, curve_step_after, curve_step_before, Arc, Area, Context, Line, Pie, Stack, Symbol,
};
use serde_json::Value;

/// The number a fixture holds, which is written down as its bits.
///
/// The fixture writes the numbers as their bits, because a number written
/// in text and read back may not be the same number: the reader may
/// round it to the nearest one, and the nearest one is not always the
/// one that was written.
fn bits(value: &Value) -> u64 {
    let text = value.as_str().expect("the bits of a number");
    // The bits are written down as they are in the number, most
    // significant first, and the port reads them the same way.
    let mut bits = 0u64;
    for i in 0..16 {
        let digit = u8::from_str_radix(&text[i..i + 1], 16).expect("a hex digit");
        bits = (bits << 4) | digit as u64;
    }
    bits
}

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/shape.json");
    let text = fs::read_to_string(path).expect("the fixture");
    serde_json::from_str(&text).expect("the fixture is JSON")
}

/// The points the oracle drew.
fn points() -> Vec<[f64; 2]> {
    (0..12)
        .map(|i| {
            [
                i as f64 * 37.5,
                (i as f64 * 0.7).sin() * 23.0 + i as f64 * 1.5,
            ]
        })
        .collect()
}

/// A line over the points.
fn line(curve: fn() -> Box<dyn d3::shape::Curve>) -> Line<[f64; 2]> {
    Line::new()
        .x(|p: &[f64; 2], _, _| p[0])
        .y(|p: &[f64; 2], _, _| p[1])
        .curve(curve)
}

#[test]
fn the_lines_are_the_same() {
    let f = fixture();
    let p = points();

    assert_eq!(
        line(curve_linear_default).apply(&p).as_deref(),
        Some(f["line"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_natural).apply(&p).as_deref(),
        Some(f["lineNatural"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_cardinal(0.5)).apply(&p).as_deref(),
        Some(f["lineCardinal"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_cardinal(0.0)).apply(&p).as_deref(),
        Some(f["lineCardinalZero"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_cardinal_open(0.5)).apply(&p).as_deref(),
        Some(f["lineCardinalOpen"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_cardinal_closed(0.5)).apply(&p).as_deref(),
        Some(f["lineCardinalClosed"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_monotone_x).apply(&p).as_deref(),
        Some(f["lineMonotoneX"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_monotone_y).apply(&p).as_deref(),
        Some(f["lineMonotoneY"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_basis).apply(&p).as_deref(),
        Some(f["lineBasis"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_basis_open).apply(&p).as_deref(),
        Some(f["lineBasisOpen"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_basis_closed).apply(&p).as_deref(),
        Some(f["lineBasisClosed"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_bundle(0.5)).apply(&p).as_deref(),
        Some(f["lineBundle"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_bundle(1.0)).apply(&p).as_deref(),
        Some(f["lineBundleOne"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_catmull_rom(0.5)).apply(&p).as_deref(),
        Some(f["lineCatmullRom"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_catmull_rom_open(0.5)).apply(&p).as_deref(),
        Some(f["lineCatmullRomOpen"].as_str().unwrap())
    );
    assert_eq!(
        line(|| curve_catmull_rom_closed(0.5)).apply(&p).as_deref(),
        Some(f["lineCatmullRomClosed"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_step).apply(&p).as_deref(),
        Some(f["lineStep"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_step_after).apply(&p).as_deref(),
        Some(f["lineStepAfter"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_step_before).apply(&p).as_deref(),
        Some(f["lineStepBefore"].as_str().unwrap())
    );
    assert_eq!(
        line(curve_linear_closed).apply(&p).as_deref(),
        Some(f["lineLinearClosed"].as_str().unwrap())
    );
}

/// The JavaScript's default curve for a line, which is the linear one.
fn curve_linear_default() -> Box<dyn d3::shape::Curve> {
    d3::shape::curve_linear()
}

#[test]
fn a_line_with_a_break_in_it_is_the_same() {
    let f = fixture();
    let p = points();
    let line = Line::new()
        .x(|p: &[f64; 2], _, _| p[0])
        .y(|p: &[f64; 2], _, _| p[1])
        .defined(|_, i, _| i % 3 != 1);
    assert_eq!(
        line.apply(&p).as_deref(),
        Some(f["lineDefined"].as_str().unwrap())
    );
    // A line of nothing is the JavaScript's `null`.
    let empty: Vec<[f64; 2]> = Vec::new();
    assert_eq!(line.apply(&empty), None);
    assert!(f["lineEmpty"].is_null());
}

#[test]
fn the_areas_are_the_same() {
    let f = fixture();
    let p = points();
    let area = |curve: fn() -> Box<dyn d3::shape::Curve>| {
        Area::default()
            .x0(|p: &[f64; 2], _, _| p[0])
            .y0(|p: &[f64; 2], _, _| p[1] - 5.0)
            .y1(|p: &[f64; 2], _, _| p[1] + 5.0)
            .curve(curve)
    };
    assert_eq!(
        area(d3::shape::curve_linear).apply(&p).as_deref(),
        Some(f["area"].as_str().unwrap())
    );
    assert_eq!(
        area(curve_natural).apply(&p).as_deref(),
        Some(f["areaNatural"].as_str().unwrap())
    );
    assert_eq!(
        area(|| curve_cardinal(0.5)).apply(&p).as_deref(),
        Some(f["areaCardinal"].as_str().unwrap())
    );
    assert_eq!(
        area(curve_step).apply(&p).as_deref(),
        Some(f["areaStep"].as_str().unwrap())
    );

    // An area with a break in it.
    let broken = Area::default()
        .x0(|p: &[f64; 2], _, _| p[0])
        .y0(|p: &[f64; 2], _, _| p[1] - 5.0)
        .y1(|p: &[f64; 2], _, _| p[1] + 5.0)
        .defined(|_, i, _| i % 3 != 1);
    assert_eq!(
        broken.apply(&p).as_deref(),
        Some(f["areaDefined"].as_str().unwrap())
    );
    let empty: Vec<[f64; 2]> = Vec::new();
    assert_eq!(broken.apply(&empty), None);
}

#[test]
fn the_symbols_are_the_same() {
    let f = fixture();
    let symbol = |kind: fn() -> Box<dyn d3::shape::SymbolType>, size: f64| {
        Symbol::default().kind(kind).size(move || size)
    };
    assert_eq!(
        symbol(d3::shape::symbol_circle, 64.0).apply().as_deref(),
        Some(f["symbolCircle"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_cross, 64.0).apply().as_deref(),
        Some(f["symbolCross"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_diamond, 64.0).apply().as_deref(),
        Some(f["symbolDiamond"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_square, 64.0).apply().as_deref(),
        Some(f["symbolSquare"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_star, 64.0).apply().as_deref(),
        Some(f["symbolStar"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_triangle, 64.0).apply().as_deref(),
        Some(f["symbolTriangle"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_wye, 64.0).apply().as_deref(),
        Some(f["symbolWye"].as_str().unwrap())
    );
    assert_eq!(
        symbol(d3::shape::symbol_circle, 100.0).apply().as_deref(),
        Some(f["symbolSize"].as_str().unwrap())
    );
}

#[test]
fn the_pies_are_the_same() {
    let f = fixture();
    let pie = Pie::of_numbers();
    let arcs = pie.apply(&[1.0f64, 2.0, 3.0, 4.0]);
    let expected = f["pie"].as_array().expect("a list");
    assert_eq!(arcs.len(), expected.len());
    for (arc, want) in arcs.iter().zip(expected) {
        assert_eq!(
            arc.index,
            want["index"].as_u64().unwrap() as usize,
            "the draw index"
        );
        assert_eq!(arc.value.to_bits(), bits(&want["value"]), "the value");
        assert_eq!(
            arc.start_angle.to_bits(),
            bits(&want["startAngle"]),
            "the start angle"
        );
        assert_eq!(
            arc.end_angle.to_bits(),
            bits(&want["endAngle"]),
            "the end angle"
        );
        assert_eq!(
            arc.pad_angle.to_bits(),
            bits(&want["padAngle"]),
            "the pad angle"
        );
    }

    // The pie sorted by its values, the other way round.
    let sorted = Pie::of_numbers()
        .sort_values(|a, b| a - b)
        .apply(&[1.0f64, 2.0, 3.0, 4.0]);
    let expected = f["pieSortValues"].as_array().expect("a list");
    for (arc, want) in sorted.iter().zip(expected) {
        assert_eq!(arc.index, want["index"].as_u64().unwrap() as usize);
        assert_eq!(arc.value.to_bits(), bits(&want["value"]));
        assert_eq!(arc.start_angle.to_bits(), bits(&want["startAngle"]));
        assert_eq!(arc.end_angle.to_bits(), bits(&want["endAngle"]));
    }

    // The pie with a pad.
    let padded = Pie::of_numbers()
        .pad_angle(|_| 0.1)
        .apply(&[1.0f64, 2.0, 3.0, 4.0]);
    let expected = f["piePad"].as_array().expect("a list");
    for (arc, want) in padded.iter().zip(expected) {
        assert_eq!(arc.index, want["index"].as_u64().unwrap() as usize);
        assert_eq!(arc.start_angle.to_bits(), bits(&want["startAngle"]));
        assert_eq!(arc.end_angle.to_bits(), bits(&want["endAngle"]));
        assert_eq!(arc.pad_angle.to_bits(), bits(&want["padAngle"]));
    }
}

/// An arc's datum, which the JavaScript reads the fields of.
struct Slice {
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
    pad: Option<f64>,
}

impl d3::shape::ArcDatum for Slice {
    fn inner_radius(&self) -> f64 {
        self.inner
    }
    fn outer_radius(&self) -> f64 {
        self.outer
    }
    fn start_angle(&self) -> f64 {
        self.start
    }
    fn end_angle(&self) -> f64 {
        self.end
    }
    fn pad_angle(&self) -> Option<f64> {
        self.pad
    }
}

#[test]
fn the_arcs_are_the_same() {
    let f = fixture();
    let quarter = Slice {
        inner: 10.0,
        outer: 50.0,
        start: 0.0,
        end: std::f64::consts::PI / 2.0,
        pad: None,
    };
    let arc = Arc::<Slice>::default();
    assert_eq!(
        arc.apply(&quarter, 0, std::slice::from_ref(&quarter))
            .as_deref(),
        Some(f["arc"].as_str().unwrap())
    );
    let centroid = arc.centroid(&quarter, 0, std::slice::from_ref(&quarter));
    let expected = f["arcCentroid"].as_array().expect("a pair");
    assert_eq!(centroid.0.to_bits(), bits(&expected[0]));
    assert_eq!(centroid.1.to_bits(), bits(&expected[1]));

    // The arc with rounded corners, which the oracle draws with an
    // inner radius of twenty.
    let band = Slice {
        inner: 20.0,
        outer: 50.0,
        start: 0.0,
        end: std::f64::consts::PI / 2.0,
        pad: None,
    };
    let cornered = Arc::<Slice>::default().corner_radius(|_| 5.0);
    assert_eq!(
        cornered
            .apply(&band, 0, std::slice::from_ref(&band))
            .as_deref(),
        Some(f["arcCorner"].as_str().unwrap())
    );

    // The arc with a pad, which the oracle draws with an inner radius
    // of twenty as well.
    let padded = Arc::<Slice>::default().pad_angle(|_| 0.1);
    assert_eq!(
        padded
            .apply(&band, 0, std::slice::from_ref(&band))
            .as_deref(),
        Some(f["arcPad"].as_str().unwrap())
    );

    // The whole circle.
    let circle = Slice {
        inner: 0.0,
        outer: 50.0,
        start: 0.0,
        end: 2.0 * std::f64::consts::PI,
        pad: None,
    };
    let arc = Arc::<Slice>::default();
    assert_eq!(
        arc.apply(&circle, 0, std::slice::from_ref(&circle))
            .as_deref(),
        Some(f["arcCircle"].as_str().unwrap())
    );

    // The point.
    let point = Slice {
        inner: 0.0,
        outer: 0.0,
        start: 0.0,
        end: std::f64::consts::PI / 2.0,
        pad: None,
    };
    let arc = Arc::<Slice>::default();
    assert_eq!(
        arc.apply(&point, 0, std::slice::from_ref(&point))
            .as_deref(),
        Some(f["arcPoint"].as_str().unwrap())
    );
}

/// A link's datum, which the JavaScript reads `source` and `target` of.
#[derive(Clone, Copy)]
struct Ends {
    source: [f64; 2],
    target: [f64; 2],
}

impl d3::shape::LinkDatum for Ends {
    type End = [f64; 2];
    fn source(&self) -> [f64; 2] {
        self.source
    }
    fn target(&self) -> [f64; 2] {
        self.target
    }
}

#[test]
fn the_links_are_the_same() {
    let f = fixture();
    let link = d3::shape::link_horizontal::<Ends>()
        .source(|d: &Ends, _, _| d.source)
        .target(|d: &Ends, _, _| d.target)
        .x(|p: &[f64; 2], _, _| p[0])
        .y(|p: &[f64; 2], _, _| p[1]);
    let data = [Ends {
        source: [0.0, 0.0],
        target: [10.0, 10.0],
    }];
    assert_eq!(
        link.apply(&data).as_deref(),
        Some(f["linkHorizontal"].as_str().unwrap())
    );

    let link = d3::shape::link_vertical::<Ends>()
        .source(|d: &Ends, _, _| d.source)
        .target(|d: &Ends, _, _| d.target)
        .x(|p: &[f64; 2], _, _| p[0])
        .y(|p: &[f64; 2], _, _| p[1]);
    assert_eq!(
        link.apply(&data).as_deref(),
        Some(f["linkVertical"].as_str().unwrap())
    );

    let link = d3::shape::link_radial::<Ends>()
        .source(|d: &Ends, _, _| d.source)
        .target(|d: &Ends, _, _| d.target)
        .x(|p: &[f64; 2], _, _| p[0])
        .y(|p: &[f64; 2], _, _| p[1]);
    let radial = [Ends {
        source: [0.0, 0.0],
        target: [1.0, 10.0],
    }];
    assert_eq!(
        link.apply(&radial).as_deref(),
        Some(f["linkRadial"].as_str().unwrap())
    );
}

/// A stack's datum, which the JavaScript reads the keys of.
#[derive(Clone)]
struct Row {
    values: Vec<f64>,
}

impl d3::shape::StackDatum for Row {
    fn value_for(&self, key: &str) -> f64 {
        let index: usize = key.parse().unwrap();
        self.values[index]
    }
}

fn stack_data() -> Vec<Row> {
    vec![
        Row {
            values: vec![1.0, 2.0, 3.0],
        },
        Row {
            values: vec![2.0, 1.0, 2.0],
        },
        Row {
            values: vec![3.0, 3.0, 1.0],
        },
    ]
}

/// The order in which the rows are stacked.
type Order = fn(&[d3::shape::Series<Row>]) -> Vec<usize>;

fn stack_with(order: Option<Order>) -> Vec<d3::shape::Series<Row>> {
    let mut stack = Stack::<Row>::default().keys(vec!["0".into(), "1".into(), "2".into()]);
    if let Some(order) = order {
        stack = stack.order(order);
    }
    stack.apply(&stack_data())
}

#[test]
fn the_stacks_are_the_same() {
    let f = fixture();
    for (key, order) in [
        ("stack", None),
        (
            "stackOrderAscending",
            Some(d3::shape::stack_order_ascending as fn(&[d3::shape::Series<Row>]) -> Vec<usize>),
        ),
        (
            "stackOrderDescending",
            Some(d3::shape::stack_order_descending as fn(&[d3::shape::Series<Row>]) -> Vec<usize>),
        ),
        (
            "stackOrderReverse",
            Some(d3::shape::stack_order_reverse as fn(&[d3::shape::Series<Row>]) -> Vec<usize>),
        ),
        (
            "stackOrderInsideOut",
            Some(d3::shape::stack_order_inside_out as fn(&[d3::shape::Series<Row>]) -> Vec<usize>),
        ),
        (
            "stackOrderAppearance",
            Some(d3::shape::stack_order_appearance as fn(&[d3::shape::Series<Row>]) -> Vec<usize>),
        ),
    ] {
        let series = stack_with(order);
        let expected = f[key].as_array().expect("a list");
        assert_eq!(series.len(), expected.len(), "{key}");
        for (s, want) in series.iter().zip(expected) {
            assert_eq!(s.key, want["key"].as_str().unwrap(), "{key}");
            if let Some(index) = want.get("index") {
                assert_eq!(s.index, index.as_u64().unwrap() as usize, "{key}");
            }
            let points = want["points"].as_array().expect("points");
            assert_eq!(s.points.len(), points.len(), "{key}");
            for (point, want) in s.points.iter().zip(points) {
                let want = want.as_array().expect("a pair");
                assert_eq!(point.y0.to_bits(), bits(&want[0]), "{key}");
                assert_eq!(point.y1.to_bits(), bits(&want[1]), "{key}");
            }
        }
    }
}

#[test]
fn the_stack_offsets_are_the_same() {
    let f = fixture();
    for (key, offset) in [
        (
            "stackOffsetExpand",
            d3::shape::stack_offset_expand as fn(&mut [d3::shape::Series<Row>], &[usize]),
        ),
        (
            "stackOffsetSilhouette",
            d3::shape::stack_offset_silhouette as fn(&mut [d3::shape::Series<Row>], &[usize]),
        ),
        (
            "stackOffsetWiggle",
            d3::shape::stack_offset_wiggle as fn(&mut [d3::shape::Series<Row>], &[usize]),
        ),
        (
            "stackOffsetDiverging",
            d3::shape::stack_offset_diverging as fn(&mut [d3::shape::Series<Row>], &[usize]),
        ),
    ] {
        let series = Stack::<Row>::default()
            .keys(vec!["0".into(), "1".into(), "2".into()])
            .offset(offset)
            .apply(&stack_data());
        let expected = f[key].as_array().expect("a list");
        assert_eq!(series.len(), expected.len(), "{key}");
        for (s, want) in series.iter().zip(expected) {
            assert_eq!(s.key, want["key"].as_str().unwrap(), "{key}");
            let points = want["points"].as_array().expect("points");
            assert_eq!(s.points.len(), points.len(), "{key}");
            for (point, want) in s.points.iter().zip(points) {
                let want = want.as_array().expect("a pair");
                assert_eq!(point.y0.to_bits(), bits(&want[0]), "{key}");
                assert_eq!(point.y1.to_bits(), bits(&want[1]), "{key}");
            }
        }
    }
}

/// A path that keeps what it was told, for checking the context's calls.
#[derive(Default)]
struct Recorder {
    text: String,
}

impl Context for Recorder {
    fn move_to(&mut self, x: f64, y: f64) {
        self.text.push_str(&format!("M{x},{y}"));
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.text.push_str(&format!("L{x},{y}"));
    }
    fn close_path(&mut self) {
        self.text.push('Z');
    }
    fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        self.text.push_str(&format!("Q{x1},{y1},{x},{y}"));
    }
    fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.text.push_str(&format!("C{x1},{y1},{x2},{y2},{x},{y}"));
    }
    fn arc(
        &mut self,
        _x: f64,
        _y: f64,
        r: f64,
        _a0: f64,
        _a1: f64,
        _ccw: bool,
    ) -> Result<(), d3::path::PathError> {
        self.text.push_str(&format!("A{r}"));
        Ok(())
    }
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.text.push_str(&format!("R{x},{y},{w},{h}"));
    }
}

/// The path is the context the curves draw into.
#[test]
fn the_path_is_a_context() {
    let mut path = Path::default();
    path.move_to(1.0, 2.0);
    path.line_to(3.0, 4.0);
    assert_eq!(path.text(), "M1,2L3,4");
    // The trait's methods are the path's own methods, and another
    // context takes them the same way.
    let mut path2 = Path::default();
    Context::move_to(&mut path2, 1.0, 2.0);
    Context::line_to(&mut path2, 3.0, 4.0);
    assert_eq!(path2.text(), "M1,2L3,4");
    let mut recorder = Recorder::default();
    Context::move_to(&mut recorder, 1.0, 2.0);
    Context::line_to(&mut recorder, 3.0, 4.0);
    assert_eq!(recorder.text, "M1,2L3,4");
}
