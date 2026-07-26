//! Reusable visual and layout stylesheet variables and mixins.
//!
//! Style modules are returned by `stylesheet!` invocations that contain
//! variables or mixins without style rules, and can be imported by another
//! `stylesheet!` invocation with `@use`.

use std::collections::BTreeMap;

use super::{
    AlignContent, AlignItems, AlignSelf, Axes, BorderType, Borders, BoxSizing, Color, Dimension,
    Display, Edges, FlexDirection, FlexWrap, GridAutoFlow, GridLine, GridTemplateTrack,
    GridTrackSize, JustifyContent, JustifyItems, JustifySelf, LayoutSize, Length, LengthAuto,
    Modifier, Overflow, Position, StyleDeclarations, ThemeValue, TuiSize, TuiSpacing, ZIndex,
};

/// A typed value stored in a reusable stylesheet module.
#[derive(Clone, Debug, PartialEq)]
pub enum StyleValue {
    /// Foreground or background color, either literal or theme-backed.
    Color(ThemeValue<Color>),
    /// Text modifier flags.
    Modifier(Modifier),
    /// Widget border sides.
    Borders(Borders),
    /// Widget border glyph set.
    BorderType(BorderType),
    /// Internal widget padding.
    Spacing(TuiSpacing),
    /// Terminal-cell image render size.
    Size(TuiSize),
    /// Layout display strategy.
    Display(Display),
    /// Authored-size box model.
    BoxSizing(BoxSizing),
    /// Horizontal and vertical overflow behavior.
    Overflow(Axes<Overflow>),
    /// Preferred, minimum, or maximum layout size.
    LayoutSize(LayoutSize<Dimension>),
    /// Margin or inset edge values.
    LengthAutoEdges(Edges<LengthAuto>),
    /// Horizontal and vertical child gaps.
    Gap(Axes<Length>),
    /// Flexbox main-axis direction.
    FlexDirection(FlexDirection),
    /// Flexbox wrapping behavior.
    FlexWrap(FlexWrap),
    /// Dimension used as a flex basis.
    Dimension(Dimension),
    /// Flex growth or shrink factor.
    Number(f32),
    /// Child cross-axis alignment.
    AlignItems(AlignItems),
    /// Item cross-axis alignment.
    AlignSelf(AlignSelf),
    /// Cross-axis content distribution.
    AlignContent(AlignContent),
    /// Child inline-axis alignment.
    JustifyItems(JustifyItems),
    /// Item inline-axis alignment.
    JustifySelf(JustifySelf),
    /// Main-axis or inline-axis content distribution.
    JustifyContent(JustifyContent),
    /// Grid automatic-flow behavior.
    GridAutoFlow(GridAutoFlow),
    /// Explicit grid row or column template.
    GridTemplateTracks(Vec<GridTemplateTrack>),
    /// Automatic grid row or column sizing functions.
    GridAutoTracks(Vec<GridTrackSize>),
    /// Grid row or column placement.
    GridLine(GridLine),
    /// Positioning scheme.
    Position(Position),
    /// Positioned stacking level.
    ZIndex(ZIndex),
}

impl StyleValue {
    /// Returns the value kind used in runtime panic messages.
    fn kind(&self) -> &'static str {
        match self {
            Self::Color(_) => "color",
            Self::Modifier(_) => "modifier",
            Self::Borders(_) => "borders",
            Self::BorderType(_) => "border_type",
            Self::Spacing(_) => "spacing",
            Self::Size(_) => "size",
            Self::Display(_) => "display",
            Self::BoxSizing(_) => "box_sizing",
            Self::Overflow(_) => "overflow",
            Self::LayoutSize(_) => "layout_size",
            Self::LengthAutoEdges(_) => "length_auto_edges",
            Self::Gap(_) => "gap",
            Self::FlexDirection(_) => "flex_direction",
            Self::FlexWrap(_) => "flex_wrap",
            Self::Dimension(_) => "dimension",
            Self::Number(_) => "number",
            Self::AlignItems(_) => "align_items",
            Self::AlignSelf(_) => "align_self",
            Self::AlignContent(_) => "align_content",
            Self::JustifyItems(_) => "justify_items",
            Self::JustifySelf(_) => "justify_self",
            Self::JustifyContent(_) => "justify_content",
            Self::GridAutoFlow(_) => "grid_auto_flow",
            Self::GridTemplateTracks(_) => "grid_template_tracks",
            Self::GridAutoTracks(_) => "grid_auto_tracks",
            Self::GridLine(_) => "grid_line",
            Self::Position(_) => "position",
            Self::ZIndex(_) => "z_index",
        }
    }
}

impl From<Color> for StyleValue {
    /// Creates a color style value from a literal color.
    ///
    /// # Arguments
    ///
    /// * `value` — Literal color to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the color.
    fn from(value: Color) -> Self {
        Self::Color(value.into())
    }
}

impl From<ThemeValue<Color>> for StyleValue {
    /// Creates a color style value from a theme-aware color.
    ///
    /// # Arguments
    ///
    /// * `value` — Theme-aware color to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the color.
    fn from(value: ThemeValue<Color>) -> Self {
        Self::Color(value)
    }
}

impl From<Modifier> for StyleValue {
    /// Creates a modifier style value.
    ///
    /// # Arguments
    ///
    /// * `value` — Text modifier flags to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the modifier flags.
    fn from(value: Modifier) -> Self {
        Self::Modifier(value)
    }
}

impl From<Borders> for StyleValue {
    /// Creates a border visibility style value.
    ///
    /// # Arguments
    ///
    /// * `value` — Border sides to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the border sides.
    fn from(value: Borders) -> Self {
        Self::Borders(value)
    }
}

impl From<BorderType> for StyleValue {
    /// Creates a border type style value.
    ///
    /// # Arguments
    ///
    /// * `value` — Border glyph set to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the border type.
    fn from(value: BorderType) -> Self {
        Self::BorderType(value)
    }
}

impl From<TuiSpacing> for StyleValue {
    /// Creates a spacing style value.
    ///
    /// # Arguments
    ///
    /// * `value` — Terminal-cell spacing to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the spacing.
    fn from(value: TuiSpacing) -> Self {
        Self::Spacing(value)
    }
}

impl From<TuiSize> for StyleValue {
    /// Creates an image size style value.
    ///
    /// # Arguments
    ///
    /// * `value` — Terminal-cell image size to store.
    ///
    /// # Returns
    ///
    /// A [`StyleValue`] containing the size.
    fn from(value: TuiSize) -> Self {
        Self::Size(value)
    }
}

macro_rules! impl_style_value_conversion {
    ($type:ty, $variant:ident, $description:literal) => {
        #[doc = concat!("Creates ", $description, " style value.")]
        ///
        /// # Arguments
        ///
        /// * `value` — Typed value to store.
        ///
        /// # Returns
        ///
        /// A [`StyleValue`] containing the value.
        impl From<$type> for StyleValue {
            fn from(value: $type) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_style_value_conversion!(Display, Display, "a layout display");
impl_style_value_conversion!(BoxSizing, BoxSizing, "a box-sizing");
impl_style_value_conversion!(Axes<Overflow>, Overflow, "an overflow");
impl_style_value_conversion!(LayoutSize<Dimension>, LayoutSize, "a layout-size");
impl_style_value_conversion!(
    Edges<LengthAuto>,
    LengthAutoEdges,
    "an automatic-length edge"
);
impl_style_value_conversion!(Axes<Length>, Gap, "a gap");
impl_style_value_conversion!(FlexDirection, FlexDirection, "a flex-direction");
impl_style_value_conversion!(FlexWrap, FlexWrap, "a flex-wrap");
impl_style_value_conversion!(Dimension, Dimension, "a dimension");
impl_style_value_conversion!(f32, Number, "a numeric");
impl_style_value_conversion!(AlignItems, AlignItems, "an align-items");
impl_style_value_conversion!(AlignSelf, AlignSelf, "an align-self");
impl_style_value_conversion!(AlignContent, AlignContent, "an align-content");
impl_style_value_conversion!(JustifyItems, JustifyItems, "a justify-items");
impl_style_value_conversion!(JustifySelf, JustifySelf, "a justify-self");
impl_style_value_conversion!(JustifyContent, JustifyContent, "a justify-content");
impl_style_value_conversion!(GridAutoFlow, GridAutoFlow, "a grid-auto-flow");
impl_style_value_conversion!(
    Vec<GridTemplateTrack>,
    GridTemplateTracks,
    "a grid-template-track list"
);
impl_style_value_conversion!(Vec<GridTrackSize>, GridAutoTracks, "a grid-auto-track list");
impl_style_value_conversion!(GridLine, GridLine, "a grid-line");
impl_style_value_conversion!(Position, Position, "a position");
impl_style_value_conversion!(ZIndex, ZIndex, "a z-index");

/// Reusable stylesheet variables and declaration mixins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleModule {
    variables: BTreeMap<String, StyleValue>,
    mixins: BTreeMap<String, StyleDeclarations>,
}

macro_rules! style_value_getter {
    ($method:ident, $variant:ident, $type:ty, $expected:literal) => {
        #[doc = concat!("Returns a stored ", $expected, " variable or panics with a stylesheet-oriented message.")]
        ///
        /// # Arguments
        ///
        /// * `name` — Variable name without the `$` prefix.
        ///
        /// # Returns
        ///
        #[doc = concat!("A [`", stringify!($type), "`] value for the stored variable.")]
        pub fn $method(&self, name: &str) -> $type {
            match self.expect_value(name) {
                StyleValue::$variant(value) => *value,
                value => panic!(
                    "stylesheet module variable `${name}` is {}, expected {}",
                    value.kind(),
                    $expected
                ),
            }
        }
    };
}

macro_rules! style_value_clone_getter {
    ($method:ident, $variant:ident, $type:ty, $expected:literal) => {
        #[doc = concat!("Returns a stored ", $expected, " variable or panics with a stylesheet-oriented message.")]
        ///
        /// # Arguments
        ///
        /// * `name` — Variable name without the `$` prefix.
        ///
        /// # Returns
        ///
        #[doc = concat!("A cloned [`", stringify!($type), "`] value for the stored variable.")]
        pub fn $method(&self, name: &str) -> $type {
            match self.expect_value(name) {
                StyleValue::$variant(value) => value.clone(),
                value => panic!(
                    "stylesheet module variable `${name}` is {}, expected {}",
                    value.kind(),
                    $expected
                ),
            }
        }
    };
}

impl StyleModule {
    /// Creates an empty style module.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a named variable and returns the updated module.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    /// * `value` — Typed stylesheet value to store.
    ///
    /// # Returns
    ///
    /// A [`StyleModule`] containing the stored variable.
    pub fn variable(mut self, name: impl Into<String>, value: impl Into<StyleValue>) -> Self {
        self.push_variable(name, value);
        self
    }

    /// Adds or replaces a named variable.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    /// * `value` — Typed stylesheet value to store.
    pub fn push_variable(&mut self, name: impl Into<String>, value: impl Into<StyleValue>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Adds or replaces a named mixin and returns the updated module.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    /// * `style` — Declaration set expanded when the mixin is included.
    ///
    /// # Returns
    ///
    /// A [`StyleModule`] containing the stored mixin.
    pub fn mixin(mut self, name: impl Into<String>, style: impl Into<StyleDeclarations>) -> Self {
        self.push_mixin(name, style);
        self
    }

    /// Adds or replaces a named mixin.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    /// * `style` — Declaration set expanded when the mixin is included.
    pub fn push_mixin(&mut self, name: impl Into<String>, style: impl Into<StyleDeclarations>) {
        self.mixins.insert(name.into(), style.into());
    }

    /// Returns a stored variable.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored [`StyleValue`] when it exists.
    pub fn get_value(&self, name: &str) -> Option<&StyleValue> {
        self.variables.get(name)
    }

    /// Returns a stored mixin.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the stored [`StyleDeclarations`] when it exists.
    pub fn get_mixin(&self, name: &str) -> Option<&StyleDeclarations> {
        self.mixins.get(name)
    }

    /// Returns a color variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`ThemeValue`] for the stored variable.
    pub fn expect_color(&self, name: &str) -> ThemeValue<Color> {
        match self.expect_value(name) {
            StyleValue::Color(value) => value.clone(),
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected color",
                value.kind()
            ),
        }
    }

    /// Returns a modifier variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`Modifier`] for the stored variable.
    pub fn expect_modifier(&self, name: &str) -> Modifier {
        match self.expect_value(name) {
            StyleValue::Modifier(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected modifier",
                value.kind()
            ),
        }
    }

    /// Returns a borders variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`Borders`] value for the stored variable.
    pub fn expect_borders(&self, name: &str) -> Borders {
        match self.expect_value(name) {
            StyleValue::Borders(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected borders",
                value.kind()
            ),
        }
    }

    /// Returns a border type variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`BorderType`] for the stored variable.
    pub fn expect_border_type(&self, name: &str) -> BorderType {
        match self.expect_value(name) {
            StyleValue::BorderType(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected border_type",
                value.kind()
            ),
        }
    }

    /// Returns a spacing variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`TuiSpacing`] value for the stored variable.
    pub fn expect_spacing(&self, name: &str) -> TuiSpacing {
        match self.expect_value(name) {
            StyleValue::Spacing(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected spacing",
                value.kind()
            ),
        }
    }

    /// Returns a size variable or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Variable name without the `$` prefix.
    ///
    /// # Returns
    ///
    /// A [`TuiSize`] value for the stored variable.
    pub fn expect_size(&self, name: &str) -> TuiSize {
        match self.expect_value(name) {
            StyleValue::Size(value) => *value,
            value => panic!(
                "stylesheet module variable `${name}` is {}, expected size",
                value.kind()
            ),
        }
    }

    style_value_getter!(expect_display, Display, Display, "display");
    style_value_getter!(expect_box_sizing, BoxSizing, BoxSizing, "box_sizing");
    style_value_getter!(expect_overflow, Overflow, Axes<Overflow>, "overflow");
    style_value_getter!(
        expect_layout_size,
        LayoutSize,
        LayoutSize<Dimension>,
        "layout_size"
    );
    style_value_getter!(
        expect_length_auto_edges,
        LengthAutoEdges,
        Edges<LengthAuto>,
        "length_auto_edges"
    );
    style_value_getter!(expect_gap, Gap, Axes<Length>, "gap");
    style_value_getter!(
        expect_flex_direction,
        FlexDirection,
        FlexDirection,
        "flex_direction"
    );
    style_value_getter!(expect_flex_wrap, FlexWrap, FlexWrap, "flex_wrap");
    style_value_getter!(expect_dimension, Dimension, Dimension, "dimension");
    style_value_getter!(expect_number, Number, f32, "number");
    style_value_getter!(expect_align_items, AlignItems, AlignItems, "align_items");
    style_value_getter!(expect_align_self, AlignSelf, AlignSelf, "align_self");
    style_value_getter!(
        expect_align_content,
        AlignContent,
        AlignContent,
        "align_content"
    );
    style_value_getter!(
        expect_justify_items,
        JustifyItems,
        JustifyItems,
        "justify_items"
    );
    style_value_getter!(
        expect_justify_self,
        JustifySelf,
        JustifySelf,
        "justify_self"
    );
    style_value_getter!(
        expect_justify_content,
        JustifyContent,
        JustifyContent,
        "justify_content"
    );
    style_value_getter!(
        expect_grid_auto_flow,
        GridAutoFlow,
        GridAutoFlow,
        "grid_auto_flow"
    );
    style_value_clone_getter!(
        expect_grid_template_tracks,
        GridTemplateTracks,
        Vec<GridTemplateTrack>,
        "grid_template_tracks"
    );
    style_value_clone_getter!(
        expect_grid_auto_tracks,
        GridAutoTracks,
        Vec<GridTrackSize>,
        "grid_auto_tracks"
    );
    style_value_getter!(expect_grid_line, GridLine, GridLine, "grid_line");
    style_value_getter!(expect_position, Position, Position, "position");
    style_value_getter!(expect_z_index, ZIndex, ZIndex, "z_index");

    /// Returns a mixin or panics with a stylesheet-oriented message.
    ///
    /// # Arguments
    ///
    /// * `name` — Mixin name.
    ///
    /// # Returns
    ///
    /// A [`StyleDeclarations`] reference for the stored mixin.
    pub fn expect_mixin(&self, name: &str) -> &StyleDeclarations {
        self.get_mixin(name)
            .unwrap_or_else(|| panic!("unknown stylesheet module mixin `{name}`"))
    }

    /// Returns a variable or panics with a stylesheet-oriented message.
    fn expect_value(&self, name: &str) -> &StyleValue {
        self.get_value(name)
            .unwrap_or_else(|| panic!("unknown stylesheet module variable `${name}`"))
    }
}
