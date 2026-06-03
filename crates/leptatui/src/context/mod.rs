//! Typed context APIs used by Leptatui apps.
//!
//! Context values are keyed by Rust type and are visible to descendant
//! Leptatui component render scopes. Values provided in an inner scope shadow
//! values of the same type from ancestor scopes.

mod hooks;
mod scope;
mod storage;

pub use hooks::{
    __with_context_scope, __with_context_scope_if_missing, expect_context, provide_context,
    use_context,
};
pub(crate) use scope::ContextScope;
