use humansize::{format_size, BINARY};

use crate::state::SharedState;
use crate::ui::models::{FileRow, RuleRow};
use crate::ui::state_ui::SharedGuiState;

pub fn sync_files(store: &gio::ListStore, state: &SharedState) {
    store.remove_all();
    let state_ref = state.borrow();
    for (idx, item) in state_ref.files.iter().enumerate() {
        let selected = state_ref.file_selected.get(idx).copied().unwrap_or(false);
        let row = FileRow::new(
            selected,
            item.is_dir,
            &item.name,
            &item.future_name,
            &item.path,
            &format_size(item.size, BINARY),
            &item.date,
        );
        store.append(&row);
    }
}

pub fn sync_rules(store: &gio::ListStore, state: &SharedState) {
    store.remove_all();
    let state_ref = state.borrow();
    for (idx, rule) in state_ref.rules.rules.iter().enumerate() {
        let selected = state_ref.rule_selected.get(idx).copied().unwrap_or(false);
        let row = RuleRow::new(
            selected,
            &crate::rule::rules::rule_type_to_string(rule.rule_type),
            &crate::rule::rules::rule_place_to_string(rule.rule_place),
            &rule.rule_description,
            crate::rule::rules::rule_type_icon(rule.rule_type),
        );
        store.append(&row);
    }
}

pub fn sync_outdated(gui_state: &SharedGuiState, state: &SharedState) {
    gui_state.borrow_mut().results_outdated = !state.borrow().rules.updated;
}
