/// Built-in default button activation count for key dispatch tests.
static MACRO_DEFAULT_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// First wrapped button activation count for component-boundary tests.
static MACRO_FIRST_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Second wrapped button activation count for component-boundary tests.
static MACRO_SECOND_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Built-in button activation count for mixed child key dispatch tests.
static MACRO_MIXED_BUILTIN_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Wrapped button activation count for mixed child key dispatch tests.
static MACRO_MIXED_WRAPPED_BUTTON_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Repeated-key handler count for key-kind filtering tests.
static MACRO_REPEAT_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
/// Release-key handler count for key-kind filtering tests.
static MACRO_RELEASE_KEY_PRESSES: AtomicUsize = AtomicUsize::new(0);
