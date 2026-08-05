//! Explorer page content rendered from page-owned signals.

use std::sync::Arc;

use leptatui::prelude::*;

use crate::{
    pages::shared::relative_path,
    services::{DirectoryListing, Workspace},
};

use super::{ExplorerList, ExplorerListProps};

/// Renders the current explorer state inside a stable scroll boundary.
///
/// # Arguments
///
/// * `workspace` — Validated workspace displayed by the page.
/// * `listing` — Page-owned directory listing signal.
/// * `selection` — Page-owned selected index signal.
/// * `error` — Page-owned recoverable error signal.
///
/// # Returns
///
/// Explorer headings and the scrollable listing.
#[component]
pub(in crate::pages::explorer) fn ExplorerContent(
    workspace: Workspace,
    listing: ArcRwSignal<DirectoryListing>,
    selection: ArcRwSignal<Option<usize>>,
    error: ArcRwSignal<Option<Arc<anyhow::Error>>>,
) -> impl IntoView {
    let home_navigate = use_navigate();
    let root = workspace.root().to_path_buf();
    let directory_workspace = workspace.clone();
    let directory_listing = listing.clone();
    let list_listing = listing.clone();
    let list_selection = selection.clone();
    let list_error = error.clone();

    stylesheet! {
        .explorer-page => {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: LayoutSize::new(
                Dimension::from(Length::percent(100.0)),
                Dimension::from(Length::percent(100.0))
            )

            @media (max-width: 60) {
                Button => { padding: TuiSpacing::ZERO }
            }

            &__title => {
                fg: Color::LightCyan,
                modifier: Modifier::BOLD
            }

            &__path => { fg: Color::LightGreen }

            &__content => {
                flex_basis: Dimension::from(Length::cells(0.0)),
                flex_grow: 1.0,
                borders: Borders::ALL,
                padding: TuiSpacing::horizontal(1),
                overflow: Axes::new(Overflow::Hidden, Overflow::Auto)

                @media (max-width: 60) {
                    padding: TuiSpacing::ZERO
                }
            }

            &__actions => {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                gap: Axes::new(Length::cells(1.0), Length::cells(0.0))

                @media (max-width: 60) {
                    flex_direction: FlexDirection::Column
                }
            }

            &__help => { fg: Color::Gray }
        }
    }

    view! {
        <Div class="explorer-page">
            <Text class="explorer-page__title">"File explorer"</Text>
            <Text class="explorer-page__path">{format!("Root: {}", root.display())}</Text>
            {move || {
                let directory = directory_listing
                    .try_get_untracked()
                    .map(|current| {
                        relative_path(directory_workspace.root(), current.directory())
                    })
                    .unwrap_or_default();
                view! {
                    <Text class="explorer-page__path">{format!("Directory: {directory}")}</Text>
                }
            }}
            <Block class="explorer-page__content">
                {move || {
                    let Some(listing) = list_listing.try_get_untracked() else {
                        return div(()).into_view();
                    };
                    let selection = list_selection.try_get_untracked().flatten();
                    let error = list_error
                        .try_get_untracked()
                        .flatten()
                        .map(|error| error.to_string());
                    view! { <ExplorerList listing=listing selection=selection error=error /> }
                        .into_view()
                }}
            </Block>
            <Div class="explorer-page__actions">
                <Button on_press=move || {
                    home_navigate("/", NavigateOptions::default());
                    AppControl::Continue
                }>"Home"</Button>
            </Div>
            <Text class="explorer-page__help">
                "↑/k ↓/j select | Enter open | ←/h parent | Esc home | q quit"
            </Text>
        </Div>
    }
}
