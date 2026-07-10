use crate::ui::state_ui::SharedGuiState;

pub fn show_overlay(gui_state: &SharedGuiState, title: &str, message: &str, indeterminate: bool) {
    log::info!("Progress: {title} - {message} (indeterminate={indeterminate})");
    let mut gs = gui_state.borrow_mut();
    gs.message_dialog_title = title.to_string();
    gs.message_dialog_text = message.to_string();
}

pub fn hide_overlay(gui_state: &SharedGuiState) {
    let mut gs = gui_state.borrow_mut();
    gs.message_dialog_title.clear();
    gs.message_dialog_text.clear();
}

pub fn show_progress_dialog(gui_state: &SharedGuiState, title: &str, message: &str, total: usize) {
    log::info!("Progress dialog: {title} - {message} (total={total})");
    let mut gs = gui_state.borrow_mut();
    gs.progress_active = true;
    gs.progress_title = title.to_string();
    gs.progress_message = message.to_string();
    gs.progress_current = 0;
    gs.progress_total = total;
}

pub fn update_progress(gui_state: &SharedGuiState, current: usize) {
    let mut gs = gui_state.borrow_mut();
    gs.progress_current = current;
}

pub fn hide_progress_dialog(gui_state: &SharedGuiState) {
    let mut gs = gui_state.borrow_mut();
    gs.progress_active = false;
    gs.progress_title.clear();
    gs.progress_message.clear();
    gs.progress_current = 0;
    gs.progress_total = 0;
}
