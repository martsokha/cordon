//! World inspector overlay (bevy_inspector_egui). Toggled with F1.
//! Pulls in `egui` + `bevy_inspector_egui` — kept behind its own
//! feature so release / lean builds don't compile those deps.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InspectorVisible(false));
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(
            WorldInspectorPlugin::new().run_if(resource_equals(InspectorVisible(true))),
        );
        app.add_systems(Update, toggle_inspector);
    }
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
struct InspectorVisible(bool);

fn toggle_inspector(keys: Res<ButtonInput<KeyCode>>, mut visible: ResMut<InspectorVisible>) {
    if keys.just_pressed(KeyCode::F1) {
        visible.0 = !visible.0;
    }
}
