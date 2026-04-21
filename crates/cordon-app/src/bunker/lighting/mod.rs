pub mod bundles;
mod flicker;
mod systems;

// `FlickerEnabled` is the scripting hook: external code flips this
// resource to turn the bunker-wide flicker on or off.
// `Flickering` is exposed so scripts can inspect / attach to
// specific lights. Both may look unused today — they're the
// external surface.
#[allow(unused_imports)]
pub use self::flicker::{FlickerEnabled, FlickerPlugin, Flickering};
pub use self::systems::spawn_lighting;
