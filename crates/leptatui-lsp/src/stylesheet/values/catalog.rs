//! Type-correct stylesheet expression templates and public enum choices.

/// One type-correct expression template.
#[derive(Clone)]
pub(super) struct Template {
    /// Visible expression without argument placeholders.
    pub(super) label: String,
    /// Rust expression containing numbered default placeholders.
    pub(super) expression: String,
    /// Short explanation of the value.
    pub(super) description: String,
}

/// Unit variants of the runtime's public layout enums.
const VARIANTS: &[(&str, &str, &str)] = &[
    ("Display", "Block", "Participates in normal block layout."),
    ("Display", "Flex", "Lays out children with flexbox."),
    ("Display", "Grid", "Lays out children with CSS Grid."),
    (
        "Display",
        "None",
        "Generates no layout or painted box for the view subtree.",
    ),
    (
        "BoxSizing",
        "ContentBox",
        "Applies authored sizes to the content box.",
    ),
    (
        "BoxSizing",
        "BorderBox",
        "Applies authored sizes to the border box.",
    ),
    (
        "Overflow",
        "Visible",
        "Paints overflowing content without clipping it.",
    ),
    (
        "Overflow",
        "Hidden",
        "Clips overflowing content while retaining scroll-container semantics.",
    ),
    (
        "Overflow",
        "Clip",
        "Clips overflowing content without creating a scroll container.",
    ),
    (
        "Overflow",
        "Scroll",
        "Always enables scrolling and reserves terminal scrollbar space.",
    ),
    (
        "Overflow",
        "Auto",
        "Enables scrolling and reserves terminal scrollbar space when needed.",
    ),
    ("FlexDirection", "Row", "Places items from left to right."),
    (
        "FlexDirection",
        "RowReverse",
        "Places items from right to left.",
    ),
    (
        "FlexDirection",
        "Column",
        "Places items from top to bottom.",
    ),
    (
        "FlexDirection",
        "ColumnReverse",
        "Places items from bottom to top.",
    ),
    ("FlexWrap", "NoWrap", "Keeps every item on one flex line."),
    (
        "FlexWrap",
        "Wrap",
        "Wraps items onto additional flex lines.",
    ),
    (
        "FlexWrap",
        "WrapReverse",
        "Wraps items with the cross-axis line order reversed.",
    ),
    (
        "AlignItems",
        "Start",
        "Aligns items to the logical cross-axis start.",
    ),
    (
        "AlignItems",
        "End",
        "Aligns items to the logical cross-axis end.",
    ),
    (
        "AlignItems",
        "FlexStart",
        "Aligns items to the flex cross-axis start.",
    ),
    (
        "AlignItems",
        "FlexEnd",
        "Aligns items to the flex cross-axis end.",
    ),
    ("AlignItems", "Center", "Centers items on the cross axis."),
    ("AlignItems", "Baseline", "Aligns item baselines."),
    (
        "AlignItems",
        "Stretch",
        "Stretches auto-sized items across the available cross axis.",
    ),
    (
        "AlignSelf",
        "Auto",
        "Uses the parent container's [`AlignItems`] value.",
    ),
    (
        "AlignSelf",
        "Start",
        "Aligns the item to the logical cross-axis start.",
    ),
    (
        "AlignSelf",
        "End",
        "Aligns the item to the logical cross-axis end.",
    ),
    (
        "AlignSelf",
        "FlexStart",
        "Aligns the item to the flex cross-axis start.",
    ),
    (
        "AlignSelf",
        "FlexEnd",
        "Aligns the item to the flex cross-axis end.",
    ),
    ("AlignSelf", "Center", "Centers the item on the cross axis."),
    ("AlignSelf", "Baseline", "Aligns the item by its baseline."),
    (
        "AlignSelf",
        "Stretch",
        "Stretches an auto-sized item across the available cross axis.",
    ),
    (
        "AlignContent",
        "Start",
        "Packs content at the logical cross-axis start.",
    ),
    (
        "AlignContent",
        "End",
        "Packs content at the logical cross-axis end.",
    ),
    (
        "AlignContent",
        "FlexStart",
        "Packs content at the flex cross-axis start.",
    ),
    (
        "AlignContent",
        "FlexEnd",
        "Packs content at the flex cross-axis end.",
    ),
    (
        "AlignContent",
        "Center",
        "Centers content on the cross axis.",
    ),
    (
        "AlignContent",
        "Stretch",
        "Stretches auto-sized content across the available cross axis.",
    ),
    (
        "AlignContent",
        "SpaceBetween",
        "Distributes free space between content groups.",
    ),
    (
        "AlignContent",
        "SpaceAround",
        "Distributes free space around content groups.",
    ),
    (
        "AlignContent",
        "SpaceEvenly",
        "Distributes equal free space around and between content groups.",
    ),
    (
        "JustifyItems",
        "Start",
        "Aligns items to the logical inline-axis start.",
    ),
    (
        "JustifyItems",
        "End",
        "Aligns items to the logical inline-axis end.",
    ),
    (
        "JustifyItems",
        "Center",
        "Centers items on the inline axis.",
    ),
    ("JustifyItems", "Baseline", "Aligns item baselines."),
    (
        "JustifyItems",
        "Stretch",
        "Stretches auto-sized items across the available inline axis.",
    ),
    (
        "JustifySelf",
        "Auto",
        "Uses the parent container's [`JustifyItems`] value.",
    ),
    (
        "JustifySelf",
        "Start",
        "Aligns the item to the logical inline-axis start.",
    ),
    (
        "JustifySelf",
        "End",
        "Aligns the item to the logical inline-axis end.",
    ),
    (
        "JustifySelf",
        "Center",
        "Centers the item on the inline axis.",
    ),
    (
        "JustifySelf",
        "Baseline",
        "Aligns the item by its baseline.",
    ),
    (
        "JustifySelf",
        "Stretch",
        "Stretches an auto-sized item across the available inline axis.",
    ),
    (
        "JustifyContent",
        "Start",
        "Packs content at the logical main-axis or inline-axis start.",
    ),
    (
        "JustifyContent",
        "End",
        "Packs content at the logical main-axis or inline-axis end.",
    ),
    (
        "JustifyContent",
        "FlexStart",
        "Packs content at the flex main-axis start.",
    ),
    (
        "JustifyContent",
        "FlexEnd",
        "Packs content at the flex main-axis end.",
    ),
    (
        "JustifyContent",
        "Center",
        "Centers content on the relevant axis.",
    ),
    (
        "JustifyContent",
        "Stretch",
        "Stretches auto-sized content across the available axis.",
    ),
    (
        "JustifyContent",
        "SpaceBetween",
        "Distributes free space between content groups.",
    ),
    (
        "JustifyContent",
        "SpaceAround",
        "Distributes free space around content groups.",
    ),
    (
        "JustifyContent",
        "SpaceEvenly",
        "Distributes equal free space around and between content groups.",
    ),
    (
        "Position",
        "Static",
        "Participates in normal flow and ignores authored insets.",
    ),
    (
        "Position",
        "Relative",
        "Participates in normal flow and offsets painting by authored insets.",
    ),
    (
        "Position",
        "Absolute",
        "Leaves normal flow and uses the nearest positioned containing block.",
    ),
    (
        "Position",
        "Fixed",
        "Leaves normal flow and uses the terminal viewport as its containing block.",
    ),
    ("Position", "Sticky", "nearest scrollport moves."),
    (
        "ZIndex",
        "Auto",
        "Uses the positioned automatic layer in source order.",
    ),
    (
        "GridAutoFlow",
        "Row",
        "Fills each row before creating another row.",
    ),
    (
        "GridAutoFlow",
        "Column",
        "Fills each column before creating another column.",
    ),
    (
        "GridAutoFlow",
        "RowDense",
        "Fills rows and backfills earlier unoccupied cells.",
    ),
    (
        "GridAutoFlow",
        "ColumnDense",
        "Fills columns and backfills earlier unoccupied cells.",
    ),
];

/// Creates templates for one compiler value category.
///
/// # Arguments
///
/// * `category` — Shared compiler category.
///
/// # Returns
///
/// A list of valid expressions with snippet defaults.
pub(super) fn templates(category: &str) -> Vec<Template> {
    let mut output = Vec::new();
    let mut add = |label: &str, expression: &str, description: &str| {
        output.push(Template {
            label: label.into(),
            expression: expression.into(),
            description: description.into(),
        });
    };
    for &(ty, variant, description) in VARIANTS {
        if ty == category && category != "Overflow" {
            let path = format!("{ty}::{variant}");
            add(&path, &path, description);
        }
    }
    match category {
        "Color" => {
            for variant in [
                "Reset",
                "Black",
                "Red",
                "Green",
                "Yellow",
                "Blue",
                "Magenta",
                "Cyan",
                "Gray",
                "DarkGray",
                "LightRed",
                "LightGreen",
                "LightYellow",
                "LightBlue",
                "LightMagenta",
                "LightCyan",
                "White",
            ] {
                let path = format!("Color::{variant}");
                add(&path, &path, "Terminal palette color.");
            }
            add(
                "Color::Rgb",
                "Color::Rgb(${1:255}, ${2:255}, ${3:255})",
                "RGB color; red, green, and blue channels range from 0 to 255.",
            );
            add(
                "Color::Indexed",
                "Color::Indexed(${1:0})",
                "Index in the terminal's 256-color palette.",
            );
        }
        "Modifier" | "Borders" => {
            let flags: &[&str] = if category == "Modifier" {
                &[
                    "BOLD",
                    "DIM",
                    "ITALIC",
                    "UNDERLINED",
                    "SLOW_BLINK",
                    "RAPID_BLINK",
                    "REVERSED",
                    "HIDDEN",
                    "CROSSED_OUT",
                ]
            } else {
                &["NONE", "ALL", "TOP", "RIGHT", "BOTTOM", "LEFT"]
            };
            for flag in flags {
                let path = format!("{category}::{flag}");
                add(&path, &path, "Combine flags with `|`.");
            }
            if category == "Modifier" {
                add("Modifier::empty", "Modifier::empty()", "No text modifiers.");
            }
        }
        "BorderType" => {
            for variant in [
                "Plain",
                "Rounded",
                "Double",
                "Thick",
                "LightDoubleDashed",
                "HeavyDoubleDashed",
                "LightTripleDashed",
                "HeavyTripleDashed",
                "LightQuadrupleDashed",
                "HeavyQuadrupleDashed",
                "QuadrantInside",
                "QuadrantOutside",
            ] {
                let path = format!("BorderType::{variant}");
                add(&path, &path, "Border drawing style.");
            }
        }
        "Spacing" => {
            add("TuiSpacing::ZERO", "TuiSpacing::ZERO", "No padding.");
            add(
                "TuiSpacing::uniform",
                "TuiSpacing::uniform(${1:1})",
                "Equal padding in terminal cells on all sides.",
            );
            add(
                "TuiSpacing::horizontal",
                "TuiSpacing::horizontal(${1:1})",
                "Left and right padding in terminal cells.",
            );
            add(
                "TuiSpacing::vertical",
                "TuiSpacing::vertical(${1:1})",
                "Top and bottom padding in terminal cells.",
            );
            add(
                "TuiSpacing::new",
                "TuiSpacing::new(${1:1}, ${2:1}, ${3:1}, ${4:1})",
                "Padding in cells: left, right, top, bottom.",
            );
        }
        "Size" => {
            add(
                "TuiSize::ZERO",
                "TuiSize::ZERO",
                "Zero image width and height.",
            );
            add(
                "TuiSize::new",
                "TuiSize::new(${1:1}, ${2:1})",
                "Image width and height in terminal cells.",
            );
        }
        "Overflow" => {
            for variant in ["Visible", "Hidden", "Clip", "Scroll", "Auto"] {
                let expression = format!("Axes::all(Overflow::{variant})");
                add(
                    &expression,
                    &expression,
                    "Apply overflow behavior to both axes.",
                );
            }
            add(
                "Axes::new",
                "Axes::new(${1:Overflow::Hidden}, ${2:Overflow::Auto})",
                "Independent horizontal and vertical overflow.",
            );
        }
        "Dimension" => {
            for variant in ["Auto", "MinContent", "MaxContent"] {
                let path = format!("Dimension::{variant}");
                add(
                    &path,
                    &path,
                    "Layout dimension; intrinsic min/max currently behave like Auto.",
                );
            }
            for method in ["cells", "percent", "vw", "vh", "vmin", "vmax"] {
                add(
                    &format!("Dimension::Length (Length::{method})"),
                    &format!("Dimension::Length(Length::{method}(${{1:1.0}}))"),
                    "Absolute cells or percentage of the containing block or viewport.",
                );
            }
            add(
                "Dimension::FitContent",
                "Dimension::FitContent(Length::cells(${1:1.0}))",
                "Currently behaves like its contained length.",
            );
        }
        "LayoutSize" => {
            add(
                "LayoutSize::all",
                "LayoutSize::all(${1:Dimension::Auto})",
                "Apply one dimension to width and height.",
            );
            add(
                "LayoutSize::new",
                "LayoutSize::new(${1:Dimension::Length(Length::percent(100.0))}, ${2:Dimension::Auto})",
                "Independent width and height dimensions.",
            );
        }
        "LengthAutoEdges" => {
            add(
                "Edges::all",
                "Edges::all(${1:LengthAuto::Auto})",
                "Apply one automatic or definite length to every edge.",
            );
            add(
                "Edges::symmetric",
                "Edges::symmetric(${1:LengthAuto::Length(Length::cells(1.0))}, ${2:LengthAuto::Auto})",
                "Horizontal and vertical edge lengths.",
            );
            add(
                "Edges::new",
                "Edges::new(${1:LengthAuto::Auto}, ${2:LengthAuto::Auto}, ${3:LengthAuto::Auto}, ${4:LengthAuto::Auto})",
                "Top, right, bottom, and left edge lengths.",
            );
        }
        "Gap" => {
            add(
                "Axes::all",
                "Axes::all(Length::cells(${1:1.0}))",
                "Equal horizontal and vertical gaps in cells.",
            );
            add(
                "Axes::new",
                "Axes::new(${1:Length::cells(1.0)}, ${2:Length::cells(1.0)})",
                "Independent horizontal and vertical gap lengths.",
            );
        }
        "Number" => {
            add("0.0", "${1:0.0}", "Floating-point property value.");
            add("1.0", "${1:1.0}", "Floating-point property value.");
        }
        "GridAutoTracks" | "GridTemplateTracks" => {
            let wrap = |track: &str| {
                if category == "GridTemplateTracks" {
                    format!("vec![GridTemplateTrack::Track({track})]")
                } else {
                    format!("vec![{track}]")
                }
            };
            for variant in ["Auto", "MinContent", "MaxContent"] {
                add(
                    &format!("vec![GridTrackSize::{variant}]"),
                    &wrap(&format!("GridTrackSize::{variant}")),
                    "Grid track sizing list.",
                );
            }
            add(
                "vec![GridTrackSize::Length]",
                &wrap("GridTrackSize::Length(Length::cells(${1:1.0}))"),
                "Fixed grid track in terminal cells.",
            );
            add(
                "vec![GridTrackSize::Fraction]",
                &wrap("GridTrackSize::Fraction(Fraction::new(${1:1.0}))"),
                "Fraction of the remaining grid space.",
            );
            add(
                "vec![GridTrackSize::minmax]",
                &wrap(
                    "GridTrackSize::minmax(${1:GridMinTrackSize::Auto}, ${2:GridMaxTrackSize::Fraction(Fraction::new(1.0))})",
                ),
                "Minimum and maximum grid track bounds.",
            );
            if category == "GridTemplateTracks" {
                add(
                    "vec![GridTemplateTrack::repeat]",
                    "vec![GridTemplateTrack::repeat(${1:GridRepeat::count(2)}, vec![${2:GridTrackSize::Auto}])]",
                    "Repeated grid-template fragment.",
                );
            }
        }
        "GridLine" => {
            add(
                "GridLine::new",
                "GridLine::new(${1:GridPlacement::line(1)}, ${2:GridPlacement::span(1)})",
                "Start and end grid edges; lines may be negative, spans count tracks.",
            );
        }
        "ZIndex" => {
            add(
                "ZIndex::Integer",
                "ZIndex::Integer(${1:0})",
                "Signed stacking level for positioned elements.",
            );
        }
        _ => {}
    }
    output
}
