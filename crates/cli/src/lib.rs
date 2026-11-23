//! Shared run operations. The `ilvuno` CLI and the `ilvunod` API both call these, so a run behaves the same
//! whichever door it was started through.

pub mod ops;
pub mod providers;
