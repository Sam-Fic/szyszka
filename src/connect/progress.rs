use crate::ui::state_ui::SharedGuiState;

pub fn show_overlay(gui_state: &SharedGuiState, title: &str, message: &str, indeterminate: bool) {
    log::info!("Progress: {} - {} (indeterminate={})", title, message, indeterminate);
    let mut gs = gui_state.borrow_mut();
    gs.message_dialog_title = title.to_string();
    gs.message_dialog_text = message.to_string();
}

pub fn hide_overlay(gui_state: &SharedGuiState) {
    let mut gs = gui_state.borrow_mut();
    gs.message_dialog_title.clear();
    gs.message_dialog_text.clear();
}
