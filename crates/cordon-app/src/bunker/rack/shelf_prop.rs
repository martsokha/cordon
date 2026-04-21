//! Maps item categories to the visual prop shown on a rack shelf.
//!
//! Most categories fall back to the generic crate ([`Prop::Box01`])
//! until dedicated models land (weapon case, medkit, etc.). Ammo
//! uses the toolbox model as a stand-in — it reads as a stackable
//! ammo crate on a shelf.

use cordon_core::item::ItemCategory;

use crate::bunker::geometry::Prop;

/// Extension trait on [`ItemCategory`] that picks the prop model
/// to display when this category of item sits on a rack shelf.
pub trait ShelfProp {
    fn shelf_prop(&self) -> Prop;
}

impl ShelfProp for ItemCategory {
    fn shelf_prop(&self) -> Prop {
        match self {
            ItemCategory::Ammo => Prop::Toolbox1,
            _ => Prop::Box01,
        }
    }
}
