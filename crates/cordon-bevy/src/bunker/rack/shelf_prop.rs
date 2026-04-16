//! Maps item categories to the visual prop shown on a rack shelf.
//!
//! Everything is [`Prop::Box01`] for now. When category-specific
//! models land (ammo crate, medical kit, weapon case), add the
//! mapping here — every shelf visual routes through this trait.

use cordon_core::item::ItemCategory;

use crate::bunker::geometry::Prop;

/// Extension trait on [`ItemCategory`] that picks the prop model
/// to display when this category of item sits on a rack shelf.
#[allow(dead_code)]
pub trait ShelfProp {
    fn shelf_prop(&self) -> Prop;
}

impl ShelfProp for ItemCategory {
    fn shelf_prop(&self) -> Prop {
        // TODO: per-category models once art lands
        // (WeaponCase, AmmoCrate, MedKit, etc.).
        let _ = self;
        Prop::Box01
    }
}
