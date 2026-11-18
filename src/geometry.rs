//! Small value types for colours and geometry in logical pixels.
//!
//! They keep the core independent of any UI framework; each editor converts
//! them to its framework's types when drawing.

use std::ops::{Add, Sub};

/// An sRGB colour with straight alpha; every channel is in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Color = Color::from_rgb(1.0, 1.0, 1.0);
    pub const TRANSPARENT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.0);

    /// An opaque colour from `0.0..=1.0` channels.
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    /// An opaque colour from `0..=255` channels.
    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Color {
        Color::from_rgba8(r, g, b, 1.0)
    }

    /// A colour from `0..=255` channels and an alpha in `0.0..=1.0`.
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: f32) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    /// The same colour with its alpha multiplied by `factor`.
    pub fn scale_alpha(self, factor: f32) -> Color {
        Color {
            a: self.a * factor,
            ..self
        }
    }

    /// The channels as `[r, g, b, a]`.
    pub fn into_rgba8(self) -> [u8; 4] {
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        [
            channel(self.r),
            channel(self.g),
            channel(self.b),
            channel(self.a),
        ]
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Color::from_rgb(r, g, b)
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Color::from_rgba(r, g, b, a)
    }
}

/// A position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    pub fn distance(self, other: Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl Add<Vector> for Point {
    type Output = Point;

    fn add(self, vector: Vector) -> Point {
        Point::new(self.x + vector.x, self.y + vector.y)
    }
}

impl Sub<Vector> for Point {
    type Output = Point;

    fn sub(self, vector: Vector) -> Point {
        Point::new(self.x - vector.x, self.y - vector.y)
    }
}

impl Sub<Point> for Point {
    type Output = Vector;

    fn sub(self, other: Point) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }
}

/// A displacement.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

impl Vector {
    pub const ZERO: Vector = Vector { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Vector {
        Vector { x, y }
    }
}

impl Add for Vector {
    type Output = Vector;

    fn add(self, other: Vector) -> Vector {
        Vector::new(self.x + other.x, self.y + other.y)
    }
}

/// A width and a height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Size = Size {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Size {
        Size { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    pub const fn new(top_left: Point, size: Size) -> Rectangle {
        Rectangle {
            x: top_left.x,
            y: top_left.y,
            width: size.width,
            height: size.height,
        }
    }

    /// A rectangle at the origin.
    pub const fn with_size(size: Size) -> Rectangle {
        Rectangle::new(Point::ORIGIN, size)
    }

    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether `point` lies inside (the right and bottom edges excluded).
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Whether the two rectangles overlap.
    pub fn intersects(&self, other: &Rectangle) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }

    /// The rectangle grown by `amount` on every side.
    pub fn expand(&self, amount: f32) -> Rectangle {
        Rectangle {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + 2.0 * amount,
            height: self.height + 2.0 * amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_convert_to_bytes() {
        assert_eq!(Color::from_rgb8(255, 0, 51).into_rgba8(), [255, 0, 51, 255]);
    }

    #[test]
    fn rectangles() {
        let rect = Rectangle::new(Point::new(10.0, 20.0), Size::new(30.0, 40.0));
        assert!(rect.contains(Point::new(10.0, 20.0)));
        assert!(!rect.contains(Point::new(40.0, 20.0)));
        assert_eq!(rect.center(), Point::new(25.0, 40.0));
        assert!(rect.intersects(&Rectangle::new(Point::new(39.0, 59.0), Size::new(5.0, 5.0))));
    }
}
