//! The UI-independent core of a Word-style WYSIWYG CommonMark editor.
//!
//! This crate holds everything an editor does apart from showing it: the
//! document model, Markdown import and export, layout and pagination,
//! editing, selection, undo, source mode and tables. It depends on no UI
//! framework and ships no fonts, icons or translations.
//!
//! A UI crate connects it to a framework:
//!
//! 1. Create an [`EditorCore`] with the [`Fonts`] the UI draws with.
//! 2. Turn framework events into [`Input`]s and pass them to
//!    [`EditorCore::update`].
//! 3. Carry out the returned [`Effect`]s and report the [`Event`]s.
//! 4. Replay [`EditorCore::content_scene`], a list of [`Op`]s, onto the
//!    framework's canvas.
//!
//! ```
//! use markdown_editor_core::{Command, EditorCore, Fonts};
//! use std::sync::Arc;
//!
//! # fn fonts() -> Arc<Fonts> {
//! #     let read = |name: &str| -> &'static [u8] {
//! #         let path = format!("{}/tests/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
//! #         Box::leak(std::fs::read(path).unwrap().into_boxed_slice())
//! #     };
//! #     Arc::new(Fonts::new([
//! #         read("DejaVuSans.ttf"),
//! #         read("DejaVuSans-Bold.ttf"),
//! #         read("DejaVuSans-Oblique.ttf"),
//! #         read("DejaVuSans-BoldOblique.ttf"),
//! #         read("DejaVuSansMono.ttf"),
//! #         read("DejaVuSansMono-Bold.ttf"),
//! #     ]))
//! # }
//! let mut editor = EditorCore::with_fonts("Hello", fonts());
//! editor.execute(Command::SelectAll);
//! editor.execute(Command::ToggleBold);
//! assert_eq!(editor.markdown(), "**Hello**\n");
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod doc;
pub mod draw;
pub mod editor;
pub mod extension;
pub mod features;
pub mod fonts;
pub mod geometry;
pub mod highlight;
pub mod images;
pub mod keyboard;
pub mod layout;
pub mod markdown;
pub mod page;
pub mod render;
pub mod style;
pub mod zoom;

#[cfg(feature = "syntect")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntect")))]
pub mod syntect;

pub use doc::{BlockKind, ColumnAlign};
pub use draw::{Frame, Image, Op, Path, PathSegment, PlacedGlyph, TextRun, Transform};
pub use editor::{
    Command, DEFAULT_TAB_WIDTH, EditorCore, Effect, Event, Formatting, ImeText, Input, LinkDialog,
    Mode, Update,
};
pub use extension::{BlockRenderer, RenderRequest};
pub use features::Features;
pub use fonts::{Face, Fonts};
pub use geometry::{Color, Point, Rectangle, Size, Vector};
pub use highlight::{CodeHighlighter, HighlightRequest, HighlightSpan};
pub use images::{
    ImageId, ImageLoader, ImageMode, ImageRejection, ImageSettings, ImageSource, PastedImage,
};
pub use keyboard::{Key, Modifiers, Named, ScrollDelta};
pub use markdown::normalize;
pub use page::{IsoSeries, Margins, Measure, Orientation, PageSize, Paper, ParseError, Unit};
pub use style::{EditorStyle, Palette, Typography};
pub use zoom::{KeyBinding, ZoomSettings, ZoomShortcuts};

#[cfg(feature = "syntect")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntect")))]
pub use crate::syntect::SyntectHighlighter;

/// The DejaVu fonts of the test suite (they are not part of the crate).
#[cfg(test)]
pub(crate) fn test_fonts() -> std::sync::Arc<Fonts> {
    static FONTS: std::sync::OnceLock<std::sync::Arc<Fonts>> = std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            std::sync::Arc::new(Fonts::new([
                include_bytes!("../tests/fonts/DejaVuSans.ttf"),
                include_bytes!("../tests/fonts/DejaVuSans-Bold.ttf"),
                include_bytes!("../tests/fonts/DejaVuSans-Oblique.ttf"),
                include_bytes!("../tests/fonts/DejaVuSans-BoldOblique.ttf"),
                include_bytes!("../tests/fonts/DejaVuSansMono.ttf"),
                include_bytes!("../tests/fonts/DejaVuSansMono-Bold.ttf"),
            ]))
        })
        .clone()
}
