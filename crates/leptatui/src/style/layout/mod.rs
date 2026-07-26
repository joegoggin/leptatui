//! Typed values for web-inspired terminal layout.
//!
//! These engine-independent values form the public layout vocabulary used by
//! stylesheets, computed layout, painting, and interaction. Terminal cells are
//! the absolute unit while percentages, viewport units, intrinsic dimensions,
//! automatic values, and grid fractions remain representable until layout.
//!
//! # Modules
//!
//! - [`alignment`] — Flexbox and grid alignment values.
//! - [`box_model`] — Display, box sizing, and overflow values.
//! - [`flex`] — Flexbox direction and wrapping values.
//! - [`geometry`] — Length, dimension, edge, axis, and size values.
//! - [`grid`] — Grid track sizing, auto-flow, and item-placement values.
//! - [`position`] — Positioning and stacking values.
//!
//! # Example
//!
//! ```
//! use leptatui::prelude::*;
//!
//! let size = LayoutSize::new(
//!     Dimension::from(Length::percent(100.0)),
//!     Dimension::FitContent(Length::vh(50.0)),
//! );
//! let insets = Edges::symmetric(LengthAuto::Auto, Length::cells(1.0).into());
//! let overflow = Axes::new(Overflow::Hidden, Overflow::Auto);
//!
//! assert_eq!(size.width, Dimension::Length(Length::Percent(100.0)));
//! assert_eq!(insets.top, LengthAuto::Length(Length::Cells(1.0)));
//! assert_eq!(overflow.y, Overflow::Auto);
//! ```

mod alignment;
mod box_model;
mod flex;
mod geometry;
mod grid;
mod position;

pub use alignment::{
    AlignContent, AlignItems, AlignSelf, JustifyContent, JustifyItems, JustifySelf,
};
pub use box_model::{BoxSizing, Display, Overflow};
pub use flex::{FlexDirection, FlexWrap};
pub use geometry::{Axes, Dimension, Edges, Fraction, LayoutSize, Length, LengthAuto};
pub use grid::{
    GridAutoFlow, GridLine, GridMaxTrackSize, GridMinTrackSize, GridPlacement, GridRepeat,
    GridTemplateTrack, GridTrackSize,
};
pub use position::{Position, ZIndex};
