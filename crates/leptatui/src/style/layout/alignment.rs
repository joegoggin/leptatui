//! Item, self, and content alignment values for flexbox and grid layout.

/// Cross-axis alignment applied by a container to its children.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignItems {
    /// Aligns items to the logical cross-axis start.
    Start,
    /// Aligns items to the logical cross-axis end.
    End,
    /// Aligns items to the flex cross-axis start.
    FlexStart,
    /// Aligns items to the flex cross-axis end.
    FlexEnd,
    /// Centers items on the cross axis.
    Center,
    /// Aligns item baselines.
    Baseline,
    /// Stretches auto-sized items across the available cross axis.
    #[default]
    Stretch,
}

/// Cross-axis alignment selected by an individual item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignSelf {
    /// Uses the parent container's [`AlignItems`] value.
    #[default]
    Auto,
    /// Aligns the item to the logical cross-axis start.
    Start,
    /// Aligns the item to the logical cross-axis end.
    End,
    /// Aligns the item to the flex cross-axis start.
    FlexStart,
    /// Aligns the item to the flex cross-axis end.
    FlexEnd,
    /// Centers the item on the cross axis.
    Center,
    /// Aligns the item by its baseline.
    Baseline,
    /// Stretches an auto-sized item across the available cross axis.
    Stretch,
}

/// Distribution of flex lines or grid tracks on the block or cross axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignContent {
    /// Packs content at the logical cross-axis start.
    Start,
    /// Packs content at the logical cross-axis end.
    End,
    /// Packs content at the flex cross-axis start.
    FlexStart,
    /// Packs content at the flex cross-axis end.
    FlexEnd,
    /// Centers content on the cross axis.
    Center,
    /// Stretches auto-sized content across the available cross axis.
    #[default]
    Stretch,
    /// Distributes free space between content groups.
    SpaceBetween,
    /// Distributes free space around content groups.
    SpaceAround,
    /// Distributes equal free space around and between content groups.
    SpaceEvenly,
}

/// Inline-axis alignment applied by a grid container to its children.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JustifyItems {
    /// Aligns items to the logical inline-axis start.
    Start,
    /// Aligns items to the logical inline-axis end.
    End,
    /// Centers items on the inline axis.
    Center,
    /// Aligns item baselines.
    Baseline,
    /// Stretches auto-sized items across the available inline axis.
    #[default]
    Stretch,
}

/// Inline-axis alignment selected by an individual grid item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JustifySelf {
    /// Uses the parent container's [`JustifyItems`] value.
    #[default]
    Auto,
    /// Aligns the item to the logical inline-axis start.
    Start,
    /// Aligns the item to the logical inline-axis end.
    End,
    /// Centers the item on the inline axis.
    Center,
    /// Aligns the item by its baseline.
    Baseline,
    /// Stretches an auto-sized item across the available inline axis.
    Stretch,
}

/// Distribution of children on a flex main axis or grid inline axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JustifyContent {
    /// Packs content at the logical main-axis or inline-axis start.
    #[default]
    Start,
    /// Packs content at the logical main-axis or inline-axis end.
    End,
    /// Packs content at the flex main-axis start.
    FlexStart,
    /// Packs content at the flex main-axis end.
    FlexEnd,
    /// Centers content on the relevant axis.
    Center,
    /// Stretches auto-sized content across the available axis.
    Stretch,
    /// Distributes free space between content groups.
    SpaceBetween,
    /// Distributes free space around content groups.
    SpaceAround,
    /// Distributes equal free space around and between content groups.
    SpaceEvenly,
}
