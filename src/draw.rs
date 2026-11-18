//! A recorded display list: the editor (and [`BlockRenderer`]s) draw into a
//! [`Frame`]; each UI replays the [`Op`]s with its framework, skipping what is
//! not visible ([`Op::is_visible`]).
//!
//! [`BlockRenderer`]: crate::BlockRenderer

use crate::fonts::{Face, Fonts};
use crate::geometry::{Color, Point, Rectangle, Size, Vector};
use std::any::Any;
use std::fmt;
use std::sync::{Arc, OnceLock};

/// A 2D affine transform: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub const fn translate(x: f32, y: f32) -> Transform {
        Transform {
            e: x,
            f: y,
            ..Transform::IDENTITY
        }
    }

    pub const fn scale(x: f32, y: f32) -> Transform {
        Transform {
            a: x,
            d: y,
            ..Transform::IDENTITY
        }
    }

    /// A rotation by `angle` radians around the origin.
    pub fn rotate(angle: f32) -> Transform {
        let (sin, cos) = angle.sin_cos();
        Transform {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// `other` applied first, then `self`.
    pub fn then_local(self, other: Transform) -> Transform {
        Transform {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    /// Where `point` ends up.
    pub fn apply(self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.e,
            self.b * point.x + self.d * point.y + self.f,
        )
    }

    /// Translation, rotation (radians) and scale of this transform, for
    /// frameworks that cannot take a matrix. Shear is not represented.
    pub fn decompose(self) -> (Vector, f32, Vector) {
        let scale_x = self.a.hypot(self.b);
        let determinant = self.a * self.d - self.b * self.c;
        let scale_y = if scale_x > 0.0 {
            determinant / scale_x
        } else {
            self.c.hypot(self.d)
        };
        (
            Vector::new(self.e, self.f),
            self.b.atan2(self.a),
            Vector::new(scale_x, scale_y),
        )
    }
}

/// A glyph of a [`TextRun`], at its measured position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    /// Glyph id in the run's [`Face`].
    pub id: u32,
    pub x: f32,
    /// The baseline.
    pub y: f32,
}

/// One line of text in one style.
///
/// It carries both forms frameworks need: the text with the top-left of its
/// line box, and every glyph placed where the layout measured it.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub content: String,
    /// Top-left of the line box.
    pub origin: Point,
    pub line_height: f32,
    pub face: Face,
    /// Font size in pixels.
    pub size: f32,
    pub color: Color,
    /// The visible glyphs (whitespace has none).
    pub glyphs: Vec<PlacedGlyph>,
}

/// One recorded drawing operation. Bounds (in the frame's coordinates) are
/// `None` when unknown, e.g. under a transform; such operations are always
/// replayed.
#[derive(Debug, Clone)]
pub enum Op {
    Rect {
        rect: Rectangle,
        color: Color,
    },
    Fill {
        path: Path,
        color: Color,
        bounds: Option<Rectangle>,
    },
    Stroke {
        path: Path,
        width: f32,
        color: Color,
        bounds: Option<Rectangle>,
    },
    Text {
        run: TextRun,
        bounds: Option<Rectangle>,
    },
    Image {
        rect: Rectangle,
        image: Image,
    },
    /// One operation drawn under a transform.
    Transformed {
        transform: Transform,
        op: Box<Op>,
    },
    /// Operations clipped to `rect`, which `transform` applies to. The
    /// operations carry their own transforms.
    Clip {
        rect: Rectangle,
        transform: Transform,
        ops: Vec<Op>,
    },
    /// Operations culled together by their common bounds.
    Group {
        bounds: Rectangle,
        ops: Vec<Op>,
    },
}

impl Op {
    /// The area this operation paints, when known.
    pub fn bounds(&self) -> Option<Rectangle> {
        match self {
            Op::Rect { rect, .. } | Op::Image { rect, .. } => Some(*rect),
            Op::Fill { bounds, .. } | Op::Stroke { bounds, .. } | Op::Text { bounds, .. } => {
                *bounds
            }
            Op::Group { bounds, .. } => Some(*bounds),
            Op::Transformed { .. } | Op::Clip { .. } => None,
        }
    }

    /// Whether the operation may paint inside `visible` (`None`: everything
    /// is visible).
    pub fn is_visible(&self, visible: Option<Rectangle>) -> bool {
        match (visible, self.bounds()) {
            (Some(visible), Some(bounds)) => visible.intersects(&bounds.expand(1.0)),
            _ => true,
        }
    }
}

/// The face of text drawn with [`Frame::fill_text`].
pub type Font = Face;

/// Horizontal alignment of [`Text`] relative to its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical alignment of [`Text`]'s line box relative to its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextBaseline {
    /// The position is the top of the line box.
    #[default]
    Top,
    Middle,
    Bottom,
}

/// A line of text for [`Frame::fill_text`].
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub content: String,
    /// Top-left of the line box (see [`Text::align_x`] and [`Text::align_y`]).
    pub position: Point,
    pub color: Color,
    /// Font size in pixels.
    pub size: f32,
    /// Height of the line box; `None` = 1.3 × size.
    pub line_height: Option<f32>,
    pub font: Font,
    pub align_x: TextAlign,
    pub align_y: TextBaseline,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            content: String::new(),
            position: Point::ORIGIN,
            color: Color::BLACK,
            size: 16.0,
            line_height: None,
            font: Font::default(),
            align_x: TextAlign::Left,
            align_y: TextBaseline::Top,
        }
    }
}

impl From<&str> for Text {
    fn from(content: &str) -> Self {
        Text {
            content: content.to_string(),
            ..Text::default()
        }
    }
}

impl From<String> for Text {
    fn from(content: String) -> Self {
        Text {
            content,
            ..Text::default()
        }
    }
}

/// How a [`Path`] is outlined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            width: 1.0,
            color: Color::BLACK,
        }
    }
}

impl Stroke {
    pub fn with_width(self, width: f32) -> Self {
        Stroke { width, ..self }
    }

    pub fn with_color(self, color: Color) -> Self {
        Stroke { color, ..self }
    }
}

/// One step of a [`Path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

/// Control point distance that makes a cubic curve follow a quarter circle.
const KAPPA: f32 = 0.552_284_8;

/// A vector shape.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path(Vec<PathSegment>);

impl Path {
    /// Builds a path with a [`PathBuilder`].
    pub fn new(build: impl FnOnce(&mut PathBuilder)) -> Path {
        let mut builder = PathBuilder(Vec::new());
        build(&mut builder);
        Path(builder.0)
    }

    pub fn line(from: Point, to: Point) -> Path {
        Path::new(|path| {
            path.move_to(from);
            path.line_to(to);
        })
    }

    pub fn rectangle(top_left: Point, size: Size) -> Path {
        Path::new(|path| path.rectangle(top_left, size))
    }

    pub fn rounded_rectangle(top_left: Point, size: Size, radius: f32) -> Path {
        let radius = radius.min(size.width / 2.0).min(size.height / 2.0).max(0.0);
        let (x0, y0) = (top_left.x, top_left.y);
        let (x1, y1) = (x0 + size.width, y0 + size.height);
        let k = radius * (1.0 - KAPPA);
        Path::new(|path| {
            path.move_to(Point::new(x0 + radius, y0));
            path.line_to(Point::new(x1 - radius, y0));
            path.bezier_curve_to(
                Point::new(x1 - k, y0),
                Point::new(x1, y0 + k),
                Point::new(x1, y0 + radius),
            );
            path.line_to(Point::new(x1, y1 - radius));
            path.bezier_curve_to(
                Point::new(x1, y1 - k),
                Point::new(x1 - k, y1),
                Point::new(x1 - radius, y1),
            );
            path.line_to(Point::new(x0 + radius, y1));
            path.bezier_curve_to(
                Point::new(x0 + k, y1),
                Point::new(x0, y1 - k),
                Point::new(x0, y1 - radius),
            );
            path.line_to(Point::new(x0, y0 + radius));
            path.bezier_curve_to(
                Point::new(x0, y0 + k),
                Point::new(x0 + k, y0),
                Point::new(x0 + radius, y0),
            );
            path.close();
        })
    }

    pub fn circle(center: Point, radius: f32) -> Path {
        Path::new(|path| path.circle(center, radius))
    }

    /// The steps of the path.
    pub fn segments(&self) -> &[PathSegment] {
        &self.0
    }

    /// A box around the path and its control points.
    pub fn bounds(&self) -> Rectangle {
        let mut points = self.0.iter().flat_map(|segment| match *segment {
            PathSegment::MoveTo(a) | PathSegment::LineTo(a) => vec![a],
            PathSegment::QuadTo(a, b) => vec![a, b],
            PathSegment::CubicTo(a, b, c) => vec![a, b, c],
            PathSegment::Close => Vec::new(),
        });
        let Some(first) = points.next() else {
            return Rectangle::default();
        };
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for point in points {
            x0 = x0.min(point.x);
            y0 = y0.min(point.y);
            x1 = x1.max(point.x);
            y1 = y1.max(point.y);
        }
        Rectangle {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }
}

/// Builds a [`Path`].
pub struct PathBuilder(Vec<PathSegment>);

impl PathBuilder {
    pub fn move_to(&mut self, point: Point) {
        self.0.push(PathSegment::MoveTo(point));
    }

    pub fn line_to(&mut self, point: Point) {
        self.0.push(PathSegment::LineTo(point));
    }

    pub fn quadratic_curve_to(&mut self, control: Point, to: Point) {
        self.0.push(PathSegment::QuadTo(control, to));
    }

    pub fn bezier_curve_to(&mut self, control_a: Point, control_b: Point, to: Point) {
        self.0.push(PathSegment::CubicTo(control_a, control_b, to));
    }

    pub fn rectangle(&mut self, top_left: Point, size: Size) {
        self.move_to(top_left);
        self.line_to(Point::new(top_left.x + size.width, top_left.y));
        self.line_to(Point::new(
            top_left.x + size.width,
            top_left.y + size.height,
        ));
        self.line_to(Point::new(top_left.x, top_left.y + size.height));
        self.close();
    }

    pub fn circle(&mut self, center: Point, radius: f32) {
        let (x, y, r) = (center.x, center.y, radius);
        let k = r * KAPPA;
        self.move_to(Point::new(x + r, y));
        self.bezier_curve_to(
            Point::new(x + r, y + k),
            Point::new(x + k, y + r),
            Point::new(x, y + r),
        );
        self.bezier_curve_to(
            Point::new(x - k, y + r),
            Point::new(x - r, y + k),
            Point::new(x - r, y),
        );
        self.bezier_curve_to(
            Point::new(x - r, y - k),
            Point::new(x - k, y - r),
            Point::new(x, y - r),
        );
        self.bezier_curve_to(
            Point::new(x + k, y - r),
            Point::new(x + r, y - k),
            Point::new(x + r, y),
        );
        self.close();
    }

    pub fn close(&mut self) {
        self.0.push(PathSegment::Close);
    }
}

struct ImageData {
    width: u32,
    height: u32,
    pixels: Arc<Vec<u8>>,
    /// The UI framework's handle of this image, created on first use.
    handle: OnceLock<Box<dyn Any + Send + Sync>>,
}

/// Decoded pixels for [`Frame::draw_image`]. Cheap to clone.
#[derive(Clone)]
pub struct Image(Arc<ImageData>);

impl Image {
    /// Straight-alpha RGBA8 pixels, `width * height * 4` bytes.
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Image {
        Image(Arc::new(ImageData {
            width,
            height,
            pixels: Arc::new(pixels),
            handle: OnceLock::new(),
        }))
    }

    pub fn width(&self) -> u32 {
        self.0.width
    }

    pub fn height(&self) -> u32 {
        self.0.height
    }

    /// The straight-alpha RGBA8 pixels.
    pub fn pixels(&self) -> &Arc<Vec<u8>> {
        &self.0.pixels
    }

    /// The UI framework's handle of this image: created by `create` on first
    /// use and kept as long as the image lives, so that the framework can
    /// cache its texture.
    ///
    /// # Panics
    ///
    /// If called with different types `T` for the same image.
    pub fn handle<T: Any + Send + Sync>(&self, create: impl FnOnce(&Image) -> T) -> &T {
        self.0
            .handle
            .get_or_init(|| Box::new(create(self)))
            .downcast_ref()
            .expect("an image is only drawn by one UI framework")
    }
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.0.width)
            .field("height", &self.0.height)
            .finish()
    }
}

/// A drawing surface that records what is drawn.
///
/// Block renderers receive one in [`BlockRenderer::draw`]; coordinates are
/// document coordinates (the bounds passed to `draw`).
///
/// [`BlockRenderer::draw`]: crate::BlockRenderer::draw
pub struct Frame {
    size: Size,
    fonts: Arc<Fonts>,
    ops: Vec<Op>,
    transform: Transform,
    saved: Vec<Transform>,
}

impl Frame {
    /// An empty frame of the given size; text is measured with `fonts`.
    pub fn new(size: Size, fonts: Arc<Fonts>) -> Frame {
        Frame {
            size,
            fonts,
            ops: Vec::new(),
            transform: Transform::IDENTITY,
            saved: Vec::new(),
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn width(&self) -> f32 {
        self.size.width
    }

    pub fn height(&self) -> f32 {
        self.size.height
    }

    pub fn center(&self) -> Point {
        Point::new(self.size.width / 2.0, self.size.height / 2.0)
    }

    /// The fonts text is measured with.
    pub fn fonts(&self) -> &Arc<Fonts> {
        &self.fonts
    }

    fn transformed(&self) -> bool {
        self.transform != Transform::IDENTITY
    }

    /// Records `op`, under the current transform if there is one.
    fn push(&mut self, op: Op) {
        if self.transformed() {
            self.ops.push(Op::Transformed {
                transform: self.transform,
                op: Box::new(op),
            });
        } else {
            self.ops.push(op);
        }
    }

    /// Bounds for culling, unless a transform makes them unknown.
    fn known(&self, bounds: Rectangle) -> Option<Rectangle> {
        (!self.transformed()).then_some(bounds)
    }

    pub fn fill_rectangle(&mut self, top_left: Point, size: Size, color: impl Into<Color>) {
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        self.push(Op::Rect {
            rect: Rectangle::new(top_left, size),
            color: color.into(),
        });
    }

    pub fn fill(&mut self, path: &Path, color: impl Into<Color>) {
        let bounds = self.known(path.bounds());
        self.push(Op::Fill {
            path: path.clone(),
            color: color.into(),
            bounds,
        });
    }

    pub fn stroke(&mut self, path: &Path, stroke: impl Into<Stroke>) {
        let stroke = stroke.into();
        let bounds = self.known(path.bounds().expand(stroke.width));
        self.push(Op::Stroke {
            path: path.clone(),
            width: stroke.width,
            color: stroke.color,
            bounds,
        });
    }

    pub fn stroke_rectangle(&mut self, top_left: Point, size: Size, stroke: impl Into<Stroke>) {
        self.stroke(&Path::rectangle(top_left, size), stroke);
    }

    /// Width of `content` in `font` at `size`.
    pub fn measure_text(&self, content: &str, font: Font, size: f32) -> f32 {
        self.fonts.width(font, content, size)
    }

    /// Draws one line of text (no wrapping).
    pub fn fill_text(&mut self, text: impl Into<Text>) {
        let text = text.into();
        if text.content.is_empty() {
            return;
        }
        let line_height = text.line_height.unwrap_or(text.size * 1.3);
        let width = self.measure_text(&text.content, text.font, text.size);
        let x = match text.align_x {
            TextAlign::Left => text.position.x,
            TextAlign::Center => text.position.x - width / 2.0,
            TextAlign::Right => text.position.x - width,
        };
        let top = match text.align_y {
            TextBaseline::Top => text.position.y,
            TextBaseline::Middle => text.position.y - line_height / 2.0,
            TextBaseline::Bottom => text.position.y - line_height,
        };
        let baseline = top
            + self
                .fonts
                .baseline_offset(text.font, text.size, line_height);

        let mut glyphs = Vec::with_capacity(text.content.len());
        let mut pen = x;
        for c in text.content.chars() {
            if !c.is_whitespace() {
                glyphs.push(PlacedGlyph {
                    id: self.fonts.glyph_id(text.font, c),
                    x: pen,
                    y: baseline,
                });
            }
            pen += self.fonts.advance(text.font, c, text.size);
        }
        let bounds = Rectangle::new(Point::new(x, top), Size::new(width, line_height));
        self.draw_run(
            TextRun {
                content: text.content,
                origin: Point::new(x, top),
                line_height,
                face: text.font,
                size: text.size,
                color: text.color,
                glyphs,
            },
            bounds,
        );
    }

    /// Draws `image` scaled into `bounds`.
    pub fn draw_image(&mut self, bounds: Rectangle, image: &Image) {
        self.push(Op::Image {
            rect: bounds,
            image: image.clone(),
        });
    }

    /// Text placed by the caller (document text).
    pub fn draw_run(&mut self, run: TextRun, bounds: Rectangle) {
        let bounds = self.known(bounds);
        self.push(Op::Text { run, bounds });
    }

    /// Moves the origin of later drawing.
    pub fn translate(&mut self, translation: Vector) {
        self.transform = self
            .transform
            .then_local(Transform::translate(translation.x, translation.y));
    }

    /// Rotates later drawing by `angle` radians around the origin.
    pub fn rotate(&mut self, angle: f32) {
        self.transform = self.transform.then_local(Transform::rotate(angle));
    }

    /// Scales later drawing uniformly.
    pub fn scale(&mut self, scale: f32) {
        self.transform = self.transform.then_local(Transform::scale(scale, scale));
    }

    pub fn scale_nonuniform(&mut self, scale: Vector) {
        self.transform = self
            .transform
            .then_local(Transform::scale(scale.x, scale.y));
    }

    /// Runs `draw`; transforms made inside are undone afterwards.
    pub fn with_save<R>(&mut self, draw: impl FnOnce(&mut Frame) -> R) -> R {
        self.saved.push(self.transform);
        let result = draw(self);
        self.transform = self.saved.pop().unwrap_or(Transform::IDENTITY);
        result
    }

    /// Runs `draw` clipped to `region`.
    pub fn with_clip<R>(&mut self, region: Rectangle, draw: impl FnOnce(&mut Frame) -> R) -> R {
        let (ops, result) = self.record(draw);
        self.ops.push(Op::Clip {
            rect: region,
            transform: self.transform,
            ops,
        });
        result
    }

    /// Records what `draw` draws as one group, culled by `bounds`.
    pub fn group(&mut self, bounds: Rectangle, draw: impl FnOnce(&mut Frame)) {
        let (ops, ()) = self.record(draw);
        if !ops.is_empty() {
            self.ops.push(Op::Group { bounds, ops });
        }
    }

    /// The operations `draw` records, kept apart from the earlier ones.
    fn record<R>(&mut self, draw: impl FnOnce(&mut Frame) -> R) -> (Vec<Op>, R) {
        let outer = std::mem::take(&mut self.ops);
        let result = draw(self);
        (std::mem::replace(&mut self.ops, outer), result)
    }

    /// The recorded operations.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn into_ops(self) -> Vec<Op> {
        self.ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: f32, height: f32) -> Frame {
        Frame::new(Size::new(width, height), crate::test_fonts())
    }

    fn visible_count(ops: &[Op], visible: Rectangle) -> usize {
        ops.iter().filter(|op| op.is_visible(Some(visible))).count()
    }

    #[test]
    fn culls_invisible_operations() {
        let mut frame = frame(100.0, 1000.0);
        frame.fill_rectangle(Point::new(0.0, 0.0), Size::new(10.0, 10.0), Color::BLACK);
        frame.fill_rectangle(Point::new(0.0, 500.0), Size::new(10.0, 10.0), Color::BLACK);
        frame.fill_text(Text {
            content: "hello".into(),
            position: Point::new(0.0, 900.0),
            ..Text::default()
        });
        assert_eq!(frame.ops().len(), 3);
        let top = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        assert_eq!(visible_count(frame.ops(), top), 1);
    }

    #[test]
    fn transforms_are_saved_and_restored() {
        let mut frame = frame(100.0, 100.0);
        frame.with_save(|frame| {
            frame.translate(Vector::new(5.0, 5.0));
            frame.fill_rectangle(Point::ORIGIN, Size::new(1.0, 1.0), Color::BLACK);
        });
        frame.fill_rectangle(Point::ORIGIN, Size::new(1.0, 1.0), Color::BLACK);
        assert!(matches!(frame.ops()[0], Op::Transformed { .. }));
        assert!(matches!(frame.ops()[1], Op::Rect { .. }));
    }

    #[test]
    fn transformed_operations_are_never_culled() {
        let mut frame = frame(100.0, 100.0);
        frame.translate(Vector::new(0.0, 5000.0));
        frame.fill_rectangle(Point::ORIGIN, Size::new(1.0, 1.0), Color::BLACK);
        let top = Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0));
        assert_eq!(visible_count(frame.ops(), top), 1);
    }

    #[test]
    fn clips_keep_their_operations_apart() {
        let mut frame = frame(100.0, 100.0);
        frame.fill_rectangle(Point::ORIGIN, Size::new(1.0, 1.0), Color::BLACK);
        let region = Rectangle::new(Point::ORIGIN, Size::new(10.0, 10.0));
        frame.with_clip(region, |frame| {
            frame.fill_rectangle(Point::ORIGIN, Size::new(2.0, 2.0), Color::BLACK);
        });
        assert_eq!(frame.ops().len(), 2);
        let Op::Clip { ops, .. } = &frame.ops()[1] else {
            panic!("expected a clip");
        };
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn text_glyphs_follow_measured_advances() {
        let mut frame = frame(200.0, 100.0);
        frame.fill_text(Text {
            content: "ab c".into(),
            position: Point::new(5.0, 10.0),
            size: 20.0,
            ..Text::default()
        });
        let Op::Text { run, .. } = &frame.ops()[0] else {
            panic!("expected text");
        };
        // The space has no glyph but advances the pen.
        assert_eq!(run.glyphs.len(), 3);
        assert_eq!(run.glyphs[0].x, 5.0);
        let a = crate::test_fonts().advance(Face::Sans, 'a', 20.0);
        assert!((run.glyphs[1].x - (5.0 + a)).abs() < 1e-4);
    }

    #[test]
    fn transforms_compose_like_nested_drawing() {
        let moved = Transform::translate(10.0, 0.0).then_local(Transform::scale(2.0, 2.0));
        assert_eq!(moved.apply(Point::new(1.0, 1.0)), Point::new(12.0, 2.0));
        let (translation, rotation, scale) = moved.decompose();
        assert_eq!(translation, Vector::new(10.0, 0.0));
        assert_eq!(rotation, 0.0);
        assert_eq!(scale, Vector::new(2.0, 2.0));
    }

    #[test]
    fn an_image_keeps_its_framework_handle() {
        let image = Image::from_rgba(1, 1, vec![0, 0, 0, 255]);
        let first = *image.handle(|_| 7_u32);
        let second = *image.clone().handle(|_| 9_u32);
        assert_eq!((first, second), (7, 7));
    }
}
