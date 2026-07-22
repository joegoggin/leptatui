/// Parent key handler count for nested key propagation tests.
static MACRO_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Child key handler count for nested key propagation tests.
static MACRO_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Parent key handler count for pass-through key propagation tests.
static MACRO_PASS_PARENT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Child key handler count for pass-through key propagation tests.
static MACRO_PASS_CHILD_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// First local key handler count for source-order dispatch tests.
static MACRO_FIRST_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
/// Second local key handler count for source-order dispatch tests.
static MACRO_SECOND_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
/// Third local key handler count for source-order dispatch tests.
static MACRO_THIRD_KEY_HANDLER: AtomicUsize = AtomicUsize::new(0);
