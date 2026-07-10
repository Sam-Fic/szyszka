use adw::prelude::*;

use super::state_ui::SharedGuiState;
use crate::state::SharedState;

pub fn show_message_dialog(window: &adw::ApplicationWindow, title: &str, message: &str) {
    let dialog = adw::AlertDialog::builder().heading(title).body(message).build();
    dialog.add_response("ok", &crate::fls!("dialog_button_ok"));
    dialog.present(Some(window));
}

pub fn show_confirm_dialog(window: &adw::ApplicationWindow, state: SharedState, gui_state: SharedGuiState, file_count: i32) {
    let dialog = adw::AlertDialog::builder()
        .heading(&crate::fls!("dialog_confirm_renaming"))
        .body(format!("{} {}", crate::fls!("renaming_question"), file_count))
        .build();
    dialog.add_response("cancel", &crate::fls!("dialog_button_cancel"));
    dialog.add_response("rename", &crate::fls!("upper_start_renaming_button"));
    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

    let window_clone = window.clone();
    dialog.connect_response(Some("rename"), move |_, _| {
        crate::connect::renaming::perform_renaming_gtk(&window_clone, &state, &gui_state);
    });
    dialog.present(Some(window));
}
