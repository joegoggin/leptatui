//! Hit-test coordinate clipping and translation.

use ratatui::layout::Rect;

/// Maps local render rectangles into terminal hit-test coordinates.
#[derive(Clone)]
pub(super) struct HitMapper {
    /// Ordered clip and translation steps from local to terminal coordinates.
    steps: Vec<HitMapStep>,
}

/// One clipping and translation step in a [`HitMapper`].
#[derive(Clone, Copy)]
struct HitMapStep {
    /// Rectangle retained before applying this step's translation.
    clip: Rect,
    /// Signed x offset applied after clipping.
    x_offset: i32,
    /// Signed y offset applied after clipping.
    y_offset: i32,
}

impl HitMapper {
    /// Creates an identity mapper for direct frame rendering.
    ///
    /// # Returns
    ///
    /// A [`HitMapper`] that preserves local coordinates without clipping.
    pub(super) const fn identity() -> Self {
        Self { steps: Vec::new() }
    }

    /// Returns a mapper extended for a clipped child buffer.
    ///
    /// The child mapping runs before retained parent steps so nested offscreen
    /// buffers preserve every clip and translation back to terminal space.
    ///
    /// # Arguments
    ///
    /// * `source` — Child-local rectangle retained from the offscreen buffer.
    /// * `target` — Parent-local rectangle receiving the retained source region.
    ///
    /// # Returns
    ///
    /// A [`HitMapper`] that maps the child through this parent mapper.
    pub(super) fn with_clipped_child(&self, source: Rect, target: Rect) -> Self {
        let child = HitMapStep {
            clip: source,
            x_offset: i32::from(target.x) - i32::from(source.x),
            y_offset: i32::from(target.y) - i32::from(source.y),
        };
        let mut steps = Vec::with_capacity(self.steps.len().saturating_add(1));
        steps.push(child);
        steps.extend_from_slice(&self.steps);
        Self { steps }
    }

    /// Maps one local rectangle into terminal coordinates.
    ///
    /// # Arguments
    ///
    /// * `area` — Rectangle expressed in the mapper's local coordinates.
    ///
    /// # Returns
    ///
    /// An [`Option`] containing the clipped and translated terminal rectangle,
    /// or [`None`] when the result is empty, outside the clip, negative, or
    /// cannot be represented by [`Rect`].
    pub(super) fn map(&self, mut area: Rect) -> Option<Rect> {
        if area.width == 0 || area.height == 0 {
            return None;
        }

        for step in &self.steps {
            area = rect_intersection(area, step.clip)?;
            let x = i32::from(area.x) + step.x_offset;
            let y = i32::from(area.y) + step.y_offset;
            if x < 0 || y < 0 {
                return None;
            }
            area.x = u16::try_from(x).ok()?;
            area.y = u16::try_from(y).ok()?;
        }

        Some(area)
    }
}

/// Returns the intersection of two terminal rectangles.
///
/// # Arguments
///
/// * `a` — First terminal rectangle to intersect.
/// * `b` — Second terminal rectangle to intersect.
///
/// # Returns
///
/// An [`Option`] containing the non-empty intersection of both rectangles.
fn rect_intersection(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));

    if right <= left || bottom <= top {
        return None;
    }

    Some(Rect {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    })
}
