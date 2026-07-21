//! UTF-8-safe cursor, word, line, and text transformation helpers.
//!
//! # Modules
//!
//! - [`cursor`] — Character-boundary and normal-mode cursor operations.
//! - [`lines`] — Logical line boundaries and vertical movement.
//! - [`paste`] — Character-wise and line-wise paste transformations.
//! - [`words`] — Vim-style word boundary movement.

mod cursor;
mod lines;
mod paste;
mod words;

pub(crate) use cursor::*;
pub(crate) use lines::*;
pub(crate) use paste::*;
pub(crate) use words::*;
