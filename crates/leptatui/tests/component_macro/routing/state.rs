/// Root route component setup count for route-switching tests.
static MACRO_ROUTE_ROOT_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Home route component setup count for route-switching tests.
static MACRO_ROUTE_HOME_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Counter route component setup count for route-switching tests.
static MACRO_ROUTE_COUNTER_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Settings route component setup count for route-switching tests.
static MACRO_ROUTE_SETTINGS_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Shared Router chrome setup count for typed parameter tests.
static MACRO_TYPED_CHROME_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
/// Matched page setup count for typed parameter tests.
static MACRO_TYPED_PAGE_SETUP_RUNS: AtomicUsize = AtomicUsize::new(0);
