/// Shared root-owned state exposed to route page branches.
#[derive(Clone, Copy)]
struct MacroSharedCount(ReadSignal<usize>);
