//! The JavaScript's transform interpolators.
//!
//! Translated from `d3-interpolate/src/transform/`, version 1.4.0, of d3
//! 5.16.0.
//!
//! The JavaScript reads a transform out of a string with a document: the
//! CSS transform is read as the browser's own computed style, and the
//! SVG transform as the browser's own matrix. The port asks for the
//! transform through [`TransformParser`], which a program implements for
//! whatever it draws with.

use crate::ieee754;
use crate::interpolate::interpolate_number;

/// A transform, as the JavaScript's own `decompose` gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// The translation along the x.
    pub translate_x: f64,
    /// The translation along the y.
    pub translate_y: f64,
    /// The rotation, in degrees.
    pub rotate: f64,
    /// The skew along the x, in degrees.
    pub skew_x: f64,
    /// The scale along the x.
    pub scale_x: f64,
    /// The scale along the y.
    pub scale_y: f64,
}

impl Default for Transform {
    fn default() -> Self {
        // The JavaScript's `identity`.
        Transform {
            translate_x: 0.0,
            translate_y: 0.0,
            rotate: 0.0,
            skew_x: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

/// A number that is drawn between, and the place in the string it goes.
type Number = (usize, Box<dyn FnMut(f64) -> f64 + 'static>);

/// The way a program reads a transform out of a string.
///
/// The JavaScript reads a CSS transform with a document, and an SVG
/// transform with a document too. A native program has no document, so
/// the port asks for the transform, and a program that has one — or that
/// can work the transform out for itself — implements this.
pub trait TransformParser {
    /// The transform the string holds, or `None` when it holds none.
    fn parse(&self, value: &str) -> Option<Transform>;
}

/// The JavaScript's `decompose`, which takes a matrix apart into the
/// parts the interpolation works with.
pub fn decompose(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Transform {
    const DEGREES: f64 = 180.0 / std::f64::consts::PI;
    let mut a = a;
    let mut b = b;
    let mut c = c;
    let mut d = d;
    let mut scale_x = (a * a + b * b).sqrt();
    if scale_x != 0.0 {
        a /= scale_x;
        b /= scale_x;
    }
    let mut skew_x = a * c + b * d;
    c -= a * skew_x;
    d -= b * skew_x;
    let scale_y = (c * c + d * d).sqrt();
    if scale_y != 0.0 {
        c /= scale_y;
        d /= scale_y;
        skew_x /= scale_y;
    }
    if a * d < b * c {
        a = -a;
        b = -b;
        skew_x = -skew_x;
        scale_x = -scale_x;
    }
    Transform {
        translate_x: e,
        translate_y: f,
        rotate: ieee754::atan2(b, a) * DEGREES,
        skew_x: ieee754::atan(skew_x) * DEGREES,
        scale_x,
        scale_y,
    }
}

/// The JavaScript's `interpolateTransformCss`, which draws between two
/// CSS transforms.
pub fn interpolate_transform_css(
    parser: &dyn TransformParser,
    a: &str,
    b: &str,
) -> Box<dyn FnMut(f64) -> String> {
    interpolate_transform(parser, a, b, "px, ", "px)", "deg)")
}

/// The JavaScript's `interpolateTransformSvg`, which draws between two
/// SVG transforms.
pub fn interpolate_transform_svg(
    parser: &dyn TransformParser,
    a: &str,
    b: &str,
) -> Box<dyn FnMut(f64) -> String> {
    interpolate_transform(parser, a, b, ", ", ")", ")")
}

/// The JavaScript's `interpolateTransform`, which draws between two
/// transforms.
pub fn interpolate_transform(
    parser: &dyn TransformParser,
    a: &str,
    b: &str,
    px_comma: &str,
    px_paren: &str,
    deg_paren: &str,
) -> Box<dyn FnMut(f64) -> String> {
    let a = parser.parse(a).unwrap_or_default();
    let b = parser.parse(b).unwrap_or_default();
    let mut s: Vec<Option<String>> = Vec::new();
    // The numbers that are drawn between, and where they go.
    let mut q: Vec<Number> = Vec::new();

    // The JavaScript's `pop`, which takes the last of the strings.
    fn pop(s: &mut Vec<Option<String>>) -> String {
        match s.pop() {
            Some(Some(text)) => format!("{} ", text),
            _ => String::new(),
        }
    }

    // The JavaScript's `translate`.
    if a.translate_x != b.translate_x || a.translate_y != b.translate_y {
        s.push(Some("translate(".to_owned()));
        s.push(None);
        s.push(Some(px_comma.to_owned()));
        s.push(None);
        s.push(Some(px_paren.to_owned()));
        let index = s.len() - 1;
        q.push((
            index - 4,
            Box::new(interpolate_number(a.translate_x, b.translate_x)),
        ));
        q.push((
            index - 2,
            Box::new(interpolate_number(a.translate_y, b.translate_y)),
        ));
    } else if b.translate_x != 0.0 || b.translate_y != 0.0 {
        s.push(Some(format!(
            "translate({}{}{}{}",
            number(b.translate_x),
            px_comma,
            number(b.translate_y),
            px_paren
        )));
    }

    // The JavaScript's `rotate`.
    let mut a_rotate = a.rotate;
    let mut b_rotate = b.rotate;
    if a_rotate != b_rotate {
        if a_rotate - b_rotate > 180.0 {
            b_rotate += 360.0;
        } else if b_rotate - a_rotate > 180.0 {
            a_rotate += 360.0;
        }
        let mut text = pop(&mut s);
        text.push_str("rotate(");
        s.push(Some(text));
        s.push(None);
        s.push(Some(deg_paren.to_owned()));
        let index = s.len() - 1;
        q.push((index - 2, Box::new(interpolate_number(a_rotate, b_rotate))));
    } else if b_rotate != 0.0 {
        let mut text = pop(&mut s);
        text.push_str("rotate(");
        text.push_str(&number(b_rotate));
        text.push_str(deg_paren);
        s.push(Some(text));
    }

    // The JavaScript's `skewX`.
    if a.skew_x != b.skew_x {
        let mut text = pop(&mut s);
        text.push_str("skewX(");
        s.push(Some(text));
        s.push(None);
        s.push(Some(deg_paren.to_owned()));
        let index = s.len() - 1;
        q.push((index - 2, Box::new(interpolate_number(a.skew_x, b.skew_x))));
    } else if b.skew_x != 0.0 {
        let mut text = pop(&mut s);
        text.push_str("skewX(");
        text.push_str(&number(b.skew_x));
        text.push_str(deg_paren);
        s.push(Some(text));
    }

    // The JavaScript's `scale`.
    if a.scale_x != b.scale_x || a.scale_y != b.scale_y {
        let mut text = pop(&mut s);
        text.push_str("scale(");
        s.push(Some(text));
        s.push(None);
        s.push(Some(",".to_owned()));
        s.push(None);
        s.push(Some(")".to_owned()));
        let index = s.len() - 1;
        q.push((
            index - 4,
            Box::new(interpolate_number(a.scale_x, b.scale_x)),
        ));
        q.push((
            index - 2,
            Box::new(interpolate_number(a.scale_y, b.scale_y)),
        ));
    } else if b.scale_x != 1.0 || b.scale_y != 1.0 {
        let mut text = pop(&mut s);
        text.push_str("scale(");
        text.push_str(&number(b.scale_x));
        text.push(',');
        text.push_str(&number(b.scale_y));
        text.push(')');
        s.push(Some(text));
    }

    let mut q = q;
    Box::new(move |t| {
        for (index, function) in q.iter_mut() {
            s[*index] = Some(function(t).to_string());
        }
        s.iter()
            .map(|part| part.clone().unwrap_or_default())
            .collect::<Vec<String>>()
            .join("")
    })
}

/// The JavaScript's own number in a string, which is the number as it is
/// written.
fn number(value: f64) -> String {
    crate::js::number_to_string(value)
}
