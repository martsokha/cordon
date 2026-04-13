use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::bunker::components::InteractPrompt;

pub(super) fn grab_cursor(mut cursor_q: Query<&mut CursorOptions>) {
    for mut cursor in &mut cursor_q {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub(super) fn hide_interact_prompt(mut prompt_q: Query<&mut Visibility, With<InteractPrompt>>) {
    for mut vis in &mut prompt_q {
        *vis = Visibility::Hidden;
    }
}
