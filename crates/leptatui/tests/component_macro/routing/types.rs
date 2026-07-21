/// Shared root-owned state exposed to route page branches.
#[derive(Clone, Copy)]
struct MacroSharedCount(ReadSignal<usize>);

/// Route values used by route-driven page switching tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MacroRoutePage {
    Home,
    Counter,
    Settings,
}
