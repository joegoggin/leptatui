//! Layout engine performance benchmarks.
//!
//! The suite exercises Leptatui's public view and render APIs so measurements
//! include style resolution, intrinsic measurement, computed layout, terminal
//! geometry retention, and painting. Parameterized fixtures protect scaling
//! behavior for deep and wide view trees.

use std::{hint::black_box, time::Duration};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use leptatui::prelude::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

/// Creates a definite border-box size.
///
/// # Arguments
///
/// * `width` — Width in terminal cells.
/// * `height` — Height in terminal cells.
///
/// # Returns
///
/// A [`TuiStyle`] containing the requested size.
fn fixture_size(width: f32, height: f32) -> TuiStyle {
    TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .size(LayoutSize::new(
            Dimension::from(Length::cells(width)),
            Dimension::from(Length::cells(height)),
        ))
}

/// Creates a fixed explicit grid track.
///
/// # Arguments
///
/// * `cells` — Track size in terminal cells.
///
/// # Returns
///
/// A [`GridTemplateTrack`] with fixed minimum and maximum sizing.
fn fixed_track(cells: f32) -> GridTemplateTrack {
    GridTemplateTrack::from(GridTrackSize::from(Length::cells(cells)))
}

/// Draws a view into an existing test terminal.
///
/// # Arguments
///
/// * `terminal` — Test terminal receiving the rendered frame.
/// * `view` — Root view rendered through the public layout path.
fn draw(terminal: &mut Terminal<TestBackend>, view: &AnyView) {
    let mut result = Ok(());

    terminal
        .draw(|frame| {
            let mut ctx = RenderCtx::new(frame);
            result = view.render(&mut ctx);
        })
        .expect("benchmark terminal draw should succeed");
    result.expect("benchmark view render should succeed");
}

/// Renders a view into a fresh test terminal.
///
/// # Arguments
///
/// * `view` — Root view rendered through the public layout path.
/// * `width` — Terminal width in cells.
/// * `height` — Terminal height in cells.
///
/// # Returns
///
/// A [`Terminal`] containing the rendered frame and retained backend buffer.
fn render_fresh(view: &AnyView, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("benchmark terminal should build");
    draw(&mut terminal, view);
    terminal
}

/// Builds a flat block-flow fixture.
///
/// # Arguments
///
/// * `items` — Number of measured text children in the root container.
///
/// # Returns
///
/// An [`AnyView`] containing the requested child collection.
fn block_fixture(items: usize) -> AnyView {
    let children = (0..items)
        .map(|index| text(format!("row {index:04}")).into_view())
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(fixture_size(120.0, 40.0).overflow(Axes::all(Overflow::Visible)))
        .into_view()
}

/// Builds a wrapped-text intrinsic measurement fixture.
///
/// # Arguments
///
/// * `paragraphs` — Number of paragraphs contributing intrinsic dimensions.
///
/// # Returns
///
/// An [`AnyView`] containing constrained semantic text.
fn intrinsic_fixture(paragraphs: usize) -> AnyView {
    let children = (0..paragraphs)
        .map(|index| {
            paragraph(format!(
                "paragraph {index:04} measures terminal-width text across several wrapped lines"
            ))
            .into_view()
        })
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(fixture_size(48.0, 40.0).overflow(Axes::all(Overflow::Visible)))
        .into_view()
}

/// Builds a recursively nested block tree.
///
/// # Arguments
///
/// * `depth` — Number of single-child layout containers surrounding the leaf.
///
/// # Returns
///
/// An [`AnyView`] whose deepest node is a measured text leaf.
fn deep_fixture(depth: usize) -> AnyView {
    let mut root = text("deep leaf").into_view();

    for _ in 0..depth {
        root = div([root])
            .with_inline_style(TuiStyle::new().overflow(Axes::all(Overflow::Visible)))
            .into_view();
    }

    root
}

/// Builds a wrapping flex collection.
///
/// # Arguments
///
/// * `items` — Number of flex items placed in the container.
///
/// # Returns
///
/// An [`AnyView`] containing fixed-size, non-shrinking flex items.
fn flex_fixture(items: usize) -> AnyView {
    let children = (0..items)
        .map(|index| {
            text(format!("{:02}", index % 100))
                .with_inline_style(fixture_size(2.0, 1.0).flex_shrink(0.0))
                .into_view()
        })
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(
            fixture_size(120.0, 60.0)
                .display(Display::Flex)
                .flex_wrap(FlexWrap::Wrap)
                .align_content(AlignContent::FlexStart)
                .align_items(AlignItems::FlexStart)
                .overflow(Axes::all(Overflow::Visible)),
        )
        .into_view()
}

/// Builds an automatically row-placed grid collection.
///
/// # Arguments
///
/// * `items` — Number of grid items placed in the container.
///
/// # Returns
///
/// An [`AnyView`] containing fixed tracks and source-ordered items.
fn grid_fixture(items: usize) -> AnyView {
    let children = (0..items)
        .map(|index| text(format!("{:02}", index % 100)).into_view())
        .collect::<Vec<_>>();

    div(children)
        .with_inline_style(
            fixture_size(120.0, 60.0)
                .display(Display::Grid)
                .grid_template_columns(vec![fixed_track(3.0); 40])
                .grid_auto_rows(vec![GridTrackSize::from(Length::cells(1.0))])
                .overflow(Axes::all(Overflow::Visible)),
        )
        .into_view()
}

/// Benchmarks cold construction of the internal computed layout tree.
///
/// # Arguments
///
/// * `criterion` — Criterion context receiving benchmark registrations.
fn layout_construction(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("layout_construction");

    for items in [10_usize, 100, 1_000] {
        group.throughput(Throughput::Elements(items as u64));
        group.bench_with_input(
            BenchmarkId::new("cold_render", items),
            &items,
            |bencher, &items| {
                bencher.iter_batched(
                    || block_fixture(items),
                    |view| black_box(render_fresh(black_box(&view), 120, 40)),
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmarks terminal text intrinsic measurement.
///
/// # Arguments
///
/// * `criterion` — Criterion context receiving benchmark registrations.
fn intrinsic_measurement(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("intrinsic_measurement");

    for paragraphs in [8_usize, 32, 128] {
        group.throughput(Throughput::Elements(paragraphs as u64));
        group.bench_with_input(
            BenchmarkId::new("wrapped_paragraphs", paragraphs),
            &paragraphs,
            |bencher, &paragraphs| {
                let view = intrinsic_fixture(paragraphs);
                bencher.iter(|| black_box(render_fresh(black_box(&view), 48, 40)));
            },
        );
    }

    group.finish();
}

/// Benchmarks retained-view recomputation across terminal resizes.
///
/// # Arguments
///
/// * `criterion` — Criterion context receiving benchmark registrations.
fn resize_recomputation(criterion: &mut Criterion) {
    let view = flex_fixture(500);
    let mut terminal =
        Terminal::new(TestBackend::new(80, 24)).expect("benchmark terminal should build");
    let mut wide = false;

    criterion.bench_function("resize_recomputation/500_flex_items", |bencher| {
        bencher.iter(|| {
            let area = if wide {
                Rect::new(0, 0, 80, 24)
            } else {
                Rect::new(0, 0, 120, 40)
            };
            wide = !wide;
            terminal
                .resize(area)
                .expect("benchmark terminal resize should succeed");
            draw(black_box(&mut terminal), black_box(&view));
            black_box(terminal.backend().buffer());
        });
    });
}

/// Benchmarks layout traversal through deeply nested containers.
///
/// # Arguments
///
/// * `criterion` — Criterion context receiving benchmark registrations.
fn deep_trees(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("deep_trees");

    for depth in [16_usize, 64, 128] {
        group.throughput(Throughput::Elements(depth as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(depth),
            &depth,
            |bencher, &depth| {
                let view = deep_fixture(depth);
                bencher.iter(|| black_box(render_fresh(black_box(&view), 80, 24)));
            },
        );
    }

    group.finish();
}

/// Benchmarks large flex and grid collections.
///
/// # Arguments
///
/// * `criterion` — Criterion context receiving benchmark registrations.
fn large_collections(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("large_collections");

    for items in [100_usize, 500, 1_000] {
        group.throughput(Throughput::Elements(items as u64));
        group.bench_with_input(
            BenchmarkId::new("flex", items),
            &items,
            |bencher, &items| {
                let view = flex_fixture(items);
                bencher.iter(|| black_box(render_fresh(black_box(&view), 120, 60)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("grid", items),
            &items,
            |bencher, &items| {
                let view = grid_fixture(items);
                bencher.iter(|| black_box(render_fresh(black_box(&view), 120, 60)));
            },
        );
    }

    group.finish();
}

/// Creates the shared Criterion configuration.
///
/// # Returns
///
/// A [`Criterion`] configured for stable local layout measurements.
fn criterion_config() -> Criterion {
    Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets =
        layout_construction,
        intrinsic_measurement,
        resize_recomputation,
        deep_trees,
        large_collections
}
criterion_main!(benches);
