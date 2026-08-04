//! Declarative, URL-like routing for terminal applications.
//!
//! [`Router`] provides an in-memory location and history stack. `Routes`
//! matches declarative route definitions, while [`Outlet`] renders nested
//! matches. Hooks expose reactive location, parameter, query, navigation, and
//! history state to descendant components.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt,
    rc::Rc,
    sync::{Arc, Mutex},
};

use leptos::prelude::{Get, GetUntracked, Memo, RwSignal, Set};
use percent_encoding::percent_decode_str;

use crate::{AnyView, AvailableSpace, Children, IntoView, LayoutSize, RenderCtx, View, keyed};

/// String map used by path-parameter and query hooks.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParamsMap(BTreeMap<String, String>);

impl ParamsMap {
    /// Returns a decoded value by name.
    ///
    /// # Arguments
    ///
    /// * `name` — Parameter or query key to read.
    ///
    /// # Returns
    ///
    /// An optional string slice containing the decoded value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// Returns all decoded entries in key order.
    ///
    /// # Returns
    ///
    /// An iterator over parameter names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// Reactive URL-like location exposed by [`use_location`].
#[derive(Clone, Copy)]
pub struct Location {
    pathname: Memo<String>,
    search: Memo<String>,
}

impl Location {
    /// Returns the reactive normalized pathname.
    ///
    /// # Returns
    ///
    /// A [`Memo<String>`] containing the pathname.
    pub const fn pathname(&self) -> Memo<String> {
        self.pathname
    }

    /// Returns the reactive search string without the leading question mark.
    ///
    /// # Returns
    ///
    /// A [`Memo<String>`] containing the search string.
    pub const fn search(&self) -> Memo<String> {
        self.search
    }
}

/// Options applied by programmatic navigation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NavigateOptions {
    /// Replaces the current history entry instead of pushing a new entry.
    pub replace: bool,
}

/// Cloneable programmatic navigation callback.
pub type Navigate = Rc<dyn Fn(&str, NavigateOptions)>;

/// In-memory router history controls.
#[derive(Clone)]
pub struct History {
    router: RouterContext,
}

impl History {
    /// Moves to the previous history entry when one exists.
    pub fn back(&self) {
        self.router.move_history(-1);
    }

    /// Moves to the next history entry when one exists.
    pub fn forward(&self) {
        self.router.move_history(1);
    }

    /// Returns whether a previous history entry exists.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether [`back`](Self::back) can move.
    pub fn can_go_back(&self) -> bool {
        self.router.revision.get();
        self.can_go_back_untracked()
    }

    /// Returns whether a previous history entry exists without tracking.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether [`back`](Self::back) can move.
    pub(crate) fn can_go_back_untracked(&self) -> bool {
        self.router
            .history
            .lock()
            .expect("router history mutex poisoned")
            .index
            > 0
    }

    /// Returns whether a following history entry exists.
    ///
    /// # Returns
    ///
    /// A [`bool`] indicating whether [`forward`](Self::forward) can move.
    pub fn can_go_forward(&self) -> bool {
        self.router.revision.get();
        let history = self
            .router
            .history
            .lock()
            .expect("router history mutex poisoned");
        history.index + 1 < history.entries.len()
    }
}

/// Shared router state stored in component context.
#[derive(Clone)]
struct RouterContext {
    history: Arc<Mutex<HistoryState>>,
    pathname: RwSignal<String>,
    search: RwSignal<String>,
    params: RwSignal<ParamsMap>,
    revision: RwSignal<u64>,
}

impl RouterContext {
    /// Creates router state from an explicit initial location.
    ///
    /// # Arguments
    ///
    /// * `initial` — Initial URL-like path and optional query string.
    ///
    /// # Returns
    ///
    /// A [`RouterContext`] initialized with one history entry.
    fn new(initial: &str) -> Self {
        let location = ParsedLocation::parse(initial, "/");
        Self {
            history: Arc::new(Mutex::new(HistoryState {
                entries: vec![location.clone()],
                index: 0,
            })),
            pathname: RwSignal::new(location.pathname.clone()),
            search: RwSignal::new(location.search.clone()),
            params: RwSignal::new(ParamsMap::default()),
            revision: RwSignal::new(0),
        }
    }

    /// Navigates to a resolved location.
    ///
    /// # Arguments
    ///
    /// * `target` — Absolute, relative, or query-only target.
    /// * `options` — History update options.
    fn navigate(&self, target: &str, options: NavigateOptions) {
        let current = self.current();
        let next = ParsedLocation::parse(target, &current.pathname);
        if next == current {
            return;
        }

        {
            let mut history = self.history.lock().expect("router history mutex poisoned");
            if options.replace {
                let index = history.index;
                history.entries[index] = next.clone();
            } else {
                let keep = history.index + 1;
                history.entries.truncate(keep);
                history.entries.push(next.clone());
                history.index = history.entries.len() - 1;
            }
        }
        self.publish(next);
    }

    /// Moves through the history stack by one signed step.
    ///
    /// # Arguments
    ///
    /// * `delta` — Negative for back and positive for forward.
    fn move_history(&self, delta: isize) {
        let location = {
            let mut history = self.history.lock().expect("router history mutex poisoned");
            let next = history.index.saturating_add_signed(delta);
            if next >= history.entries.len() || next == history.index {
                return;
            }
            history.index = next;
            history.entries[next].clone()
        };
        self.publish(location);
    }

    /// Returns the current history location.
    ///
    /// # Returns
    ///
    /// A [`ParsedLocation`] cloned from the current entry.
    fn current(&self) -> ParsedLocation {
        let history = self.history.lock().expect("router history mutex poisoned");
        history.entries[history.index].clone()
    }

    /// Publishes a history location to reactive consumers.
    ///
    /// # Arguments
    ///
    /// * `location` — Current location to expose.
    fn publish(&self, location: ParsedLocation) {
        self.pathname.set(location.pathname);
        self.search.set(location.search);
        self.revision
            .set(self.revision.get_untracked().wrapping_add(1));
    }
}

/// Internal normalized history location.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedLocation {
    /// Normalized leading-slash path.
    pathname: String,
    /// Query string without its leading question mark.
    search: String,
}

impl ParsedLocation {
    /// Parses and resolves a URL-like target.
    ///
    /// # Arguments
    ///
    /// * `target` — Absolute, relative, or query-only target.
    /// * `current_path` — Current pathname used for relative resolution.
    ///
    /// # Returns
    ///
    /// A normalized [`ParsedLocation`].
    fn parse(target: &str, current_path: &str) -> Self {
        let target = target.split('#').next().unwrap_or_default();
        let (path, search) = target
            .split_once('?')
            .map_or((target, ""), |(path, search)| (path, search));
        let pathname = if target.starts_with('?') || path.is_empty() {
            normalize_path(current_path)
        } else if path.starts_with('/') {
            normalize_path(path)
        } else {
            let base = current_path.rsplit_once('/').map_or("/", |(base, _)| base);
            normalize_path(&format!("{base}/{path}"))
        };
        Self {
            pathname,
            search: search.to_owned(),
        }
    }
}

/// In-memory history stack and cursor.
struct HistoryState {
    /// Ordered locations retained by the current process.
    entries: Vec<ParsedLocation>,
    /// Zero-based current entry index.
    index: usize,
}

/// Provides router state to descendant views.
///
/// # Arguments
///
/// * `initial_path` — Initial URL-like location.
/// * `children` — Application shell and route views.
///
/// # Returns
///
/// A routed child view tree.
#[crate::component]
#[allow(non_snake_case)]
pub fn Router(
    #[prop(default = String::from("/"), into)] initial_path: String,
    children: Children,
) -> impl IntoView {
    let router = RouterContext::new(&initial_path);
    let child_router = router.clone();
    let mut children = crate::context::hooks::__with_context_scope(|| {
        crate::context::provide_context(child_router);
        children()
    });
    let child = if children.len() == 1 {
        children.remove(0)
    } else {
        crate::div(children).into_view()
    };
    RouterView { router, child }
}

/// Layout-transparent router context provider.
struct RouterView {
    /// Shared router state exposed during descendant traversal.
    router: RouterContext,
    /// Application shell owned by the router.
    child: AnyView,
}

impl RouterView {
    /// Runs a callback while the router context is available.
    ///
    /// # Arguments
    ///
    /// * `callback` — Work that may construct or traverse routed descendants.
    ///
    /// # Returns
    ///
    /// An `R` value returned by `callback`.
    fn with_context<R>(&self, callback: impl FnOnce() -> R) -> R {
        let router = self.router.clone();
        crate::context::hooks::__with_context_scope(|| {
            crate::context::provide_context(router);
            callback()
        })
    }
}

impl View for RouterView {
    /// Renders the routed child with router context.
    ///
    /// # Arguments
    ///
    /// * `ctx` — Render context receiving the child.
    ///
    /// # Returns
    ///
    /// An empty [`crate::Result`] on success.
    fn render(&self, ctx: &mut RenderCtx<'_, '_>) -> crate::Result<()> {
        self.with_context(|| ctx.render_view(&self.child))
    }

    /// Measures the routed child with router context.
    ///
    /// # Arguments
    ///
    /// * `known_dimensions` — Exact parent dimensions.
    /// * `available_space` — Remaining layout space.
    /// * `ctx` — Render context used by the child.
    ///
    /// # Returns
    ///
    /// A [`LayoutSize`] containing the child's measurement.
    fn measure(
        &self,
        known_dimensions: LayoutSize<Option<f32>>,
        available_space: LayoutSize<AvailableSpace>,
        ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        self.with_context(|| self.child.measure(known_dimensions, available_space, ctx))
    }

    /// Returns this router view for type erasure.
    ///
    /// # Returns
    ///
    /// A shared [`std::any::Any`] reference.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns this router view mutably for type erasure.
    ///
    /// # Returns
    ///
    /// A mutable [`std::any::Any`] reference.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Visits the routed child inside router context.
    fn __visit_layout_children(
        &self,
        ctx: &mut RenderCtx<'_, '_>,
        visitor: &mut dyn FnMut(&AnyView, &mut RenderCtx<'_, '_>),
    ) {
        self.with_context(|| visitor(&self.child, ctx));
    }

    /// Returns that this wrapper does not create a layout box.
    fn __is_layout_transparent(&self) -> bool {
        true
    }

    /// Preserves router history and child state across compatible rebuilds.
    fn reconcile(&mut self, previous: &dyn View) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.router = previous.router.clone();
            self.child.reconcile_from(&previous.child);
        }
    }

    /// Returns whether the previous node is another router wrapper.
    fn can_reconcile_from(&self, previous: &dyn View) -> bool {
        previous.as_any().is::<Self>()
    }

    /// Returns the routed child slice.
    fn children(&self) -> &[AnyView] {
        std::slice::from_ref(&self.child)
    }

    /// Returns the mutable routed child slice.
    fn children_mut(&mut self) -> &mut [AnyView] {
        std::slice::from_mut(&mut self.child)
    }

    /// Dispatches custom events with router context available.
    fn __dispatch_event(
        &mut self,
        event: &crossterm::event::Event,
    ) -> crate::Result<crate::AppControl> {
        let router = self.router.clone();
        crate::context::hooks::__with_context_scope(|| {
            crate::context::provide_context(router);
            self.child.__dispatch_event(event)
        })
    }

    /// Dispatches custom key handlers with router context available.
    fn __dispatch_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> crate::Result<crate::KeyControl> {
        let router = self.router.clone();
        crate::context::hooks::__with_context_scope(|| {
            crate::context::provide_context(router);
            self.child.__dispatch_key_event(key)
        })
    }
}

/// Returns reactive location state from the nearest router.
///
/// # Returns
///
/// A [`Location`] containing reactive pathname and search memos.
///
/// # Panics
///
/// Panics if no [`Router`] exists in context.
pub fn use_location() -> Location {
    let router = use_router();
    let pathname_router = router.clone();
    let search_router = router.clone();
    Location {
        pathname: Memo::new(move |_| pathname_router.pathname.get()),
        search: Memo::new(move |_| search_router.search.get()),
    }
}

/// Returns reactive path parameters for the current match.
///
/// # Returns
///
/// A [`Memo<ParamsMap>`] containing decoded parameters.
///
/// # Panics
///
/// Panics if no [`Router`] exists in context.
pub fn use_params_map() -> Memo<ParamsMap> {
    let router = use_router();
    Memo::new(move |_| router.params.get())
}

/// Returns reactive decoded query values for the current location.
///
/// # Returns
///
/// A [`Memo<ParamsMap>`] containing query values.
///
/// # Panics
///
/// Panics if no [`Router`] exists in context.
pub fn use_query_map() -> Memo<ParamsMap> {
    let router = use_router();
    Memo::new(move |_| parse_query(&router.search.get()))
}

/// Returns a cloneable programmatic navigation callback.
///
/// # Returns
///
/// A [`Navigate`] callback accepting a target and [`NavigateOptions`].
///
/// # Panics
///
/// Panics if no [`Router`] exists in context.
pub fn use_navigate() -> Navigate {
    let router = use_router();
    Rc::new(move |target, options| router.navigate(target, options))
}

/// Returns in-memory history controls from the nearest router.
///
/// # Returns
///
/// A [`History`] value with back and forward controls.
///
/// # Panics
///
/// Panics if no [`Router`] exists in context.
pub fn use_history() -> History {
    History {
        router: use_router(),
    }
}

/// Returns in-memory history controls when a router exists in context.
///
/// # Returns
///
/// An optional [`History`] value for the nearest router.
pub(crate) fn try_use_history() -> Option<History> {
    crate::context::use_context::<RouterContext>().map(|router| History { router })
}

/// Returns the nearest router context.
///
/// # Returns
///
/// A [`RouterContext`] cloned from component context.
///
/// # Panics
///
/// Panics when called outside a router.
fn use_router() -> RouterContext {
    crate::context::expect_context::<RouterContext>()
}

/// Deferred view factory stored by route definitions.
pub type RouteViewFactory = Rc<dyn Fn() -> AnyView>;

/// Declarative route node collected by [`Routes`].
struct RouteDefinition {
    /// Static, parameterized, or wildcard path pattern.
    path: String,
    /// Deferred component or view factory.
    view: RouteViewFactory,
    /// Nested declarations rendered through an outlet.
    children: Vec<RouteDefinition>,
}

impl fmt::Debug for RouteDefinition {
    /// Formats a route definition without formatting its factory.
    ///
    /// # Arguments
    ///
    /// * `formatter` — Formatter receiving the debug structure.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating whether formatting succeeded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RouteDefinition")
            .field("path", &self.path)
            .field("children", &self.children)
            .finish()
    }
}

/// Non-rendering route declaration transported through `view!` children.
struct RouteDefinitionView {
    /// Declaration extracted by the nearest route list.
    definition: Option<RouteDefinition>,
}

impl View for RouteDefinitionView {
    /// Renders no terminal content.
    ///
    /// # Arguments
    ///
    /// * `_ctx` — Unused render context.
    ///
    /// # Returns
    ///
    /// An empty [`crate::Result`] on success.
    fn render(&self, _ctx: &mut RenderCtx<'_, '_>) -> crate::Result<()> {
        Ok(())
    }

    /// Returns zero intrinsic size.
    ///
    /// # Arguments
    ///
    /// * `_known_dimensions` — Unused exact dimensions.
    /// * `_available_space` — Unused available space.
    /// * `_ctx` — Unused render context.
    ///
    /// # Returns
    ///
    /// A zero [`LayoutSize`].
    fn measure(
        &self,
        _known_dimensions: LayoutSize<Option<f32>>,
        _available_space: LayoutSize<AvailableSpace>,
        _ctx: &mut RenderCtx<'_, '_>,
    ) -> LayoutSize<f32> {
        LayoutSize::all(0.0)
    }

    /// Returns this declaration for type erasure.
    ///
    /// # Returns
    ///
    /// A shared [`std::any::Any`] reference.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// Returns this declaration mutably for type erasure.
    ///
    /// # Returns
    ///
    /// A mutable [`std::any::Any`] reference.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Creates a non-rendering route declaration for macro expansion.
///
/// # Arguments
///
/// * `path` — Static, parameterized, or wildcard route pattern.
/// * `view` — Deferred matched view factory.
/// * `children` — Nested route declarations.
///
/// # Returns
///
/// An [`AnyView`] containing a [`RouteDefinitionView`].
#[doc(hidden)]
pub fn __route_definition(
    path: impl Into<String>,
    view: RouteViewFactory,
    children: Vec<AnyView>,
) -> AnyView {
    AnyView::new(RouteDefinitionView {
        definition: Some(RouteDefinition {
            path: path.into(),
            view,
            children: extract_definitions(children),
        }),
    })
}

/// Creates the dynamic matched route boundary for macro expansion.
///
/// # Arguments
///
/// * `fallback` — Deferred fallback view factory.
/// * `children` — Top-level route declarations.
///
/// # Returns
///
/// A dynamic routed view.
#[doc(hidden)]
pub fn __routes(fallback: RouteViewFactory, children: Vec<AnyView>) -> impl IntoView {
    let router = use_router();
    let routes = Rc::new(extract_definitions(children));
    let key_router = router.clone();
    let build_router = router.clone();
    let key_routes = Rc::clone(&routes);
    keyed(
        move || {
            key_router.revision.get_untracked();
            let matched = match_routes(&key_routes, &key_router.pathname.get_untracked());
            key_router.params.set(
                matched
                    .as_ref()
                    .map_or_else(ParamsMap::default, |matched| matched.params.clone()),
            );
            matched.map(|matched| matched.chain.first().copied().unwrap_or(usize::MAX))
        },
        move || {
            let pathname = build_router.pathname.get_untracked();
            if let Some(matched) = match_routes(&routes, &pathname) {
                build_router.params.set(matched.params.clone());
                build_match(Rc::clone(&routes), build_router.clone(), 0)
            } else {
                build_router.params.set(ParamsMap::default());
                fallback()
            }
        },
    )
}

/// Extracts route definitions from declaration views.
///
/// # Arguments
///
/// * `views` — Type-erased route declaration views.
///
/// # Returns
///
/// A vector containing the extracted definitions.
///
/// # Panics
///
/// Panics if a non-route view is nested directly under route declarations.
fn extract_definitions(views: Vec<AnyView>) -> Vec<RouteDefinition> {
    views
        .into_iter()
        .map(|mut view| {
            view.downcast_mut::<RouteDefinitionView>()
                .and_then(|definition| definition.definition.take())
                .expect("Routes and ParentRoute children must be Route declarations")
        })
        .collect()
}

thread_local! {
    /// Outlet factory active while a matched parent component is constructed.
    static OUTLET_FACTORY: RefCell<Option<RouteViewFactory>> = RefCell::new(None);
}

/// Renders the currently matched nested route.
///
/// # Returns
///
/// The nested routed view, or an empty view when the route has no child match.
#[crate::component]
#[allow(non_snake_case)]
pub fn Outlet() -> impl IntoView {
    __outlet()
}

/// Resolves the outlet active during matched parent construction.
///
/// # Returns
///
/// An [`AnyView`] containing the nested route or an empty view.
#[doc(hidden)]
pub fn __outlet() -> AnyView {
    OUTLET_FACTORY.with(|slot| {
        slot.borrow().as_ref().map_or_else(
            || crate::div(Vec::<AnyView>::new()).into_view(),
            |factory| factory(),
        )
    })
}

/// Builds one matched route and installs its nested outlet factory.
///
/// # Arguments
///
/// * `routes` — Complete top-level route tree.
/// * `router` — Active router state.
/// * `depth` — Matched-chain depth to construct.
///
/// # Returns
///
/// An [`AnyView`] containing the matched component.
fn build_match(routes: Rc<Vec<RouteDefinition>>, router: RouterContext, depth: usize) -> AnyView {
    let pathname = router.pathname.get_untracked();
    let Some(matched) = match_routes(&routes, &pathname) else {
        return crate::div(Vec::<AnyView>::new()).into_view();
    };
    let Some(_) = matched.chain.get(depth) else {
        return crate::div(Vec::<AnyView>::new()).into_view();
    };
    let route = definition_at(&routes, &matched.chain[..=depth]);
    let factory = Rc::clone(&route.view);
    let outlet = if depth + 1 < matched.chain.len() {
        let outlet_routes = Rc::clone(&routes);
        let outlet_router = router.clone();
        Some(Rc::new(move || {
            let key_routes = Rc::clone(&outlet_routes);
            let build_routes = Rc::clone(&outlet_routes);
            let key_router = outlet_router.clone();
            let build_router = outlet_router.clone();
            keyed(
                move || {
                    key_router.revision.get_untracked();
                    match_routes(&key_routes, &key_router.pathname.get_untracked())
                        .and_then(|matched| matched.chain.get(depth + 1).copied())
                },
                move || build_match(Rc::clone(&build_routes), build_router.clone(), depth + 1),
            )
            .into_view()
        }) as RouteViewFactory)
    } else {
        None
    };

    OUTLET_FACTORY.with(|slot| {
        let previous = slot.replace(outlet);
        let view = factory();
        slot.replace(previous);
        view
    })
}

/// Returns a route definition by its index chain.
///
/// # Arguments
///
/// * `routes` — Top-level route definitions.
/// * `chain` — Index chain from root to a nested definition.
///
/// # Returns
///
/// A shared [`RouteDefinition`] reference.
fn definition_at<'a>(routes: &'a [RouteDefinition], chain: &[usize]) -> &'a RouteDefinition {
    let mut definitions = routes;
    let mut current = &definitions[chain[0]];
    for index in &chain[1..] {
        definitions = &current.children;
        current = &definitions[*index];
    }
    current
}

/// Successful route match.
#[derive(Clone, Debug)]
struct RouteMatch {
    /// Indexes selecting one definition at each nesting level.
    chain: Vec<usize>,
    /// Decoded parameters accumulated across the chain.
    params: ParamsMap,
    /// Segment-specific precedence values used to rank candidates.
    score: Vec<u8>,
}

/// Finds the highest-precedence route chain for a pathname.
///
/// # Arguments
///
/// * `routes` — Route definitions to search.
/// * `pathname` — Normalized pathname.
///
/// # Returns
///
/// An optional [`RouteMatch`] containing indexes and decoded parameters.
fn match_routes(routes: &[RouteDefinition], pathname: &str) -> Option<RouteMatch> {
    let segments = split_segments(pathname);
    let mut matches = Vec::new();
    collect_matches(
        routes,
        &segments,
        0,
        Vec::new(),
        ParamsMap::default(),
        Vec::new(),
        &mut matches,
    );
    matches.into_iter().max_by(|left, right| {
        left.score
            .cmp(&right.score)
            .then_with(|| right.chain.cmp(&left.chain))
    })
}

/// Recursively collects complete route matches.
#[allow(clippy::too_many_arguments)]
fn collect_matches(
    routes: &[RouteDefinition],
    segments: &[&str],
    offset: usize,
    chain: Vec<usize>,
    params: ParamsMap,
    score: Vec<u8>,
    matches: &mut Vec<RouteMatch>,
) {
    for (index, route) in routes.iter().enumerate() {
        let Some(pattern_match) = match_pattern(&route.path, segments, offset) else {
            continue;
        };
        let mut next_chain = chain.clone();
        next_chain.push(index);
        let mut next_params = params.clone();
        next_params.0.extend(pattern_match.params.0);
        let mut next_score = score.clone();
        next_score.extend(pattern_match.score);
        let next_offset = offset + pattern_match.consumed;

        if next_offset == segments.len() {
            matches.push(RouteMatch {
                chain: next_chain.clone(),
                params: next_params.clone(),
                score: next_score.clone(),
            });
        }
        if !route.children.is_empty() {
            collect_matches(
                &route.children,
                segments,
                next_offset,
                next_chain,
                next_params,
                next_score,
                matches,
            );
        }
    }
}

/// Result of matching one route pattern at an offset.
struct PatternMatch {
    /// Number of pathname segments consumed by this pattern.
    consumed: usize,
    /// Decoded values captured by parameter and wildcard segments.
    params: ParamsMap,
    /// Static, parameter, and wildcard precedence values.
    score: Vec<u8>,
}

/// Matches a route pattern against remaining pathname segments.
///
/// # Arguments
///
/// * `pattern` — Route path pattern.
/// * `segments` — Full pathname segments.
/// * `offset` — First segment available to the pattern.
///
/// # Returns
///
/// An optional [`PatternMatch`] when every pattern segment matches.
fn match_pattern(pattern: &str, segments: &[&str], offset: usize) -> Option<PatternMatch> {
    let patterns = split_segments(pattern);
    let mut params = ParamsMap::default();
    let mut score = Vec::new();
    let mut consumed = 0;

    for (position, pattern) in patterns.iter().enumerate() {
        if let Some(name) = pattern.strip_prefix('*') {
            let value = segments[offset + consumed..].join("/");
            params.0.insert(name.to_owned(), decode(&value));
            consumed = segments.len().saturating_sub(offset);
            score.push(1);
            return Some(PatternMatch {
                consumed,
                params,
                score,
            });
        }
        let segment = *segments.get(offset + consumed)?;
        if let Some(name) = pattern.strip_prefix(':') {
            params.0.insert(name.to_owned(), decode(segment));
            score.push(2);
        } else if *pattern == segment {
            score.push(3);
        } else {
            return None;
        }
        consumed += 1;

        if position + 1 == patterns.len() {
            break;
        }
    }

    Some(PatternMatch {
        consumed,
        params,
        score,
    })
}

/// Splits a normalized path into non-empty segments.
///
/// # Arguments
///
/// * `path` — Path or route pattern to split.
///
/// # Returns
///
/// A vector of path segment slices.
fn split_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Normalizes a path by resolving empty, current, and parent segments.
///
/// # Arguments
///
/// * `path` — Path to normalize.
///
/// # Returns
///
/// A leading-slash pathname without a trailing slash except for root.
fn normalize_path(path: &str) -> String {
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    if segments.is_empty() {
        String::from("/")
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// Percent-decodes a path or query value without failing on invalid UTF-8.
///
/// # Arguments
///
/// * `value` — Encoded value to decode.
///
/// # Returns
///
/// A decoded [`String`].
fn decode(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().into_owned()
}

/// Parses a query string into a last-value-wins map.
///
/// # Arguments
///
/// * `search` — Query string without a leading question mark.
///
/// # Returns
///
/// A decoded [`ParamsMap`].
fn parse_query(search: &str) -> ParamsMap {
    let mut map = ParamsMap::default();
    for pair in search.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        map.0.insert(decode(key), decode(value));
    }
    map
}

#[cfg(test)]
/// Unit tests for route matching and location parsing.
mod tests {
    use super::*;

    /// Creates a route definition without render behavior.
    ///
    /// # Arguments
    ///
    /// * `path` — Pattern stored by the definition.
    /// * `children` — Nested route definitions.
    ///
    /// # Returns
    ///
    /// A [`RouteDefinition`] suitable for matcher tests.
    fn definition(path: &str, children: Vec<RouteDefinition>) -> RouteDefinition {
        RouteDefinition {
            path: path.to_owned(),
            view: Rc::new(|| crate::text("test").into_view()),
            children,
        }
    }

    /// Verifies static routes outrank parameters and wildcards.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// /files/new
    /// /files/:name
    /// /files/*path
    /// ```
    ///
    /// # Assertions
    ///
    /// - `/files/new` selects the static declaration.
    /// - `/files/guide` selects and decodes the parameter declaration.
    /// - `/files/docs/guide` selects the wildcard declaration.
    ///
    /// # Why
    ///
    /// Declaration order must not allow broad patterns to shadow specific
    /// routes.
    #[test]
    fn matcher_prioritizes_static_parameter_and_wildcard_routes() {
        let routes = vec![
            definition("/files/*path", Vec::new()),
            definition("/files/:name", Vec::new()),
            definition("/files/new", Vec::new()),
        ];

        assert_eq!(match_routes(&routes, "/files/new").unwrap().chain, [2]);
        let parameter = match_routes(&routes, "/files/guide%20one").unwrap();
        assert_eq!(parameter.chain, [1]);
        assert_eq!(parameter.params.get("name"), Some("guide one"));
        let wildcard = match_routes(&routes, "/files/docs/guide").unwrap();
        assert_eq!(wildcard.chain, [0]);
        assert_eq!(wildcard.params.get("path"), Some("docs/guide"));
    }

    /// Verifies nested definitions compose parent and child patterns.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ParentRoute /settings
    /// └── Route :section
    /// ```
    ///
    /// # Assertions
    ///
    /// - `/settings/profile` matches both definition indexes.
    /// - The child parameter contains `profile`.
    #[test]
    fn matcher_builds_nested_route_chains() {
        let routes = vec![definition(
            "/settings",
            vec![definition(":section", Vec::new())],
        )];

        let matched = match_routes(&routes, "/settings/profile").unwrap();
        assert_eq!(matched.chain, [0, 0]);
        assert_eq!(matched.params.get("section"), Some("profile"));
    }

    /// Verifies path normalization and last-value-wins query parsing.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// ../settings/?mode=light&mode=dark&name=Ada%20Lovelace
    /// ```
    ///
    /// # Assertions
    ///
    /// - Relative parent segments resolve against the current path.
    /// - Trailing slashes are removed.
    /// - Repeated query keys retain their final value.
    /// - Percent-encoded query values are decoded.
    #[test]
    fn locations_normalize_paths_and_decode_queries() {
        let location = ParsedLocation::parse(
            "../settings/?mode=light&mode=dark&name=Ada%20Lovelace",
            "/docs/guide",
        );
        assert_eq!(location.pathname, "/settings");
        let query = parse_query(&location.search);
        assert_eq!(query.get("mode"), Some("dark"));
        assert_eq!(query.get("name"), Some("Ada Lovelace"));
    }
}
