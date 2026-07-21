//! Typed context APIs used by Leptatui apps.
//!
//! Context values are keyed by Rust type and are visible to descendant
//! Leptatui component render scopes. Values provided in an inner scope shadow
//! values of the same type from ancestor scopes.
//!
//! # Modules
//!
//! - `hooks` — Public typed context accessors.
//! - `scope` — RAII management for render-scope context frames.
//! - `storage` — Type-erased context frame storage and lookup.

pub(crate) mod hooks;
mod scope;
mod storage;

pub use hooks::{expect_context, provide_context, use_context};
pub(crate) use scope::ContextScope;
