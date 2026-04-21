//! Cross-cutting UI concerns shared by every in-game surface
//! (laptop, dialogue panel, toasts, menus).
//!
//! - [`fonts`] — the primary UI font handle loaded once at startup.
//! - [`scale`] — resolution-driven `UiScale` so everything stays
//!   legible across 720p → 4K without per-call-site changes.
//!
//! Module-specific UI (the laptop tab panels, bunker overlays)
//! stays with its owning subsystem; only UI state that applies
//! everywhere belongs here.

pub mod fonts;
pub mod scale;

pub use self::fonts::{FontsPlugin, UiFont};
pub use self::scale::UiScalePlugin;
