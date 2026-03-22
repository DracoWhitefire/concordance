//! The [`Diagnostic`] bound shared by all warning and violation types.

use core::fmt;

/// Bound required on all warning and violation types, built-in or custom.
///
/// Blanket-implemented for any type that is [`fmt::Display`] + [`fmt::Debug`].
pub trait Diagnostic: fmt::Display + fmt::Debug {}

impl<T: fmt::Display + fmt::Debug> Diagnostic for T {}
