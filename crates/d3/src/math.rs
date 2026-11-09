//! The program's own geometry, in the reference's own numbers.
//!
//! The web reference works in ordinary numbers for its positions and
//! sizes, and so does this port: a point is two numbers and a rectangle
//! is two points. There is no dependency on a drawing toolkit's geometry
//! here, so the same numbers can be handed to egui, GPUI or anything else
//! that knows how to paint a line.

/// A point in the plane, in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A rectangle, given by its top-left and bottom-right corners.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn from_min_max(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    pub fn from_min_size(min: Point, size: (f32, f32)) -> Self {
        Self {
            min,
            max: Point::new(min.x + size.0, min.y + size.1),
        }
    }

    pub fn left(&self) -> f32 {
        self.min.x
    }
    pub fn top(&self) -> f32 {
        self.min.y
    }
    pub fn right(&self) -> f32 {
        self.max.x
    }
    pub fn bottom(&self) -> f32 {
        self.max.y
    }
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
    pub fn center(&self) -> Point {
        Point::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}
