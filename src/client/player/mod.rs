//! Client-side player input binding and visual spawn handling.

/// Attaches input maps to controlled player entities.
mod input;
/// Adjusts visuals when interpolated player entities spawn.
mod spawn;

pub(crate) use input::*;
pub(crate) use spawn::*;
