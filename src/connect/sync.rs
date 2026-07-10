use chrono::DateTime;
use gio::prelude::ListModelExt;
use glib::prelude::*;
use humansize::{format_size, BINARY};

use crate::state::SharedState;
use crate::ui::models::{FileRow, RuleRow};
use crate::ui::state_ui::SharedGuiState;

pub fn sync_files(store: &gio::ListStore, state: &SharedState) {
    let state_ref = state.borrow();
    let items: Vec<glib::Object> = state_ref
        .files
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let selected = state_ref.file_selected.get(idx).copied().unwrap_or(false);
            FileRow::new(
                selected,
                item.is_dir,
                &item.name,
                &item.future_name,
                &item.path,
                &format_size(item.size, BINARY),
                &item.date,
            )
            .upcast::<glib::Object>()
        })
        .collect();
    store.splice(0, store.n_items(), &items);
}

pub fn sync_rules(store: &gio::ListStore, state: &SharedState) {
    let state_ref = state.borrow();
    let items: Vec<glib::Object> = state_ref
        .rules
        .rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| {
            let selected = state_ref.rule_selected.get(idx).copied().unwrap_or(false);
            RuleRow::new(
                selected,
                &crate::rule::rules::rule_type_to_string(rule.rule_type),
                &crate::rule::rules::rule_place_to_string(rule.rule_place),
                &rule.rule_description,
                crate::rule::rules::rule_type_icon(rule.rule_type),
            )
            .upcast::<glib::Object>()
        })
        .collect();
    store.splice(0, store.n_items(), &items);
}

/// Read GTK MultiSelection state and update state.file_selected.
/// With SortListModel, GTK selection indices are in sorted order,
/// so we must map them back to the original ListStore order.
pub fn sync_selection_from_gtk(selection: &gtk::MultiSelection, state: &SharedState) {
    use gtk::prelude::ListModelExt;
    use gtk::prelude::SelectionModelExt;
    let mut s = state.borrow_mut();
    let n = s.files.len();
    let mut selected = vec![false; n];
    let gtk_selection = selection.selection();
    let sort_model = s.file_sort_model.as_ref();
    for i in 0..gtk_selection.size() {
        let sorted_idx = gtk_selection.nth(i as u32);
        if let Some(sm) = sort_model {
            if let Some(item) = sm.item(sorted_idx) {
                if let Some(file_row) = item.downcast_ref::<crate::ui::models::FileRow>() {
                    let row_path = file_row.path();
                    let row_name = file_row.current_name();
                    if let Some(orig_idx) = s.files.iter().position(|f| f.path == row_path && f.name == row_name) {
                        if orig_idx < n {
                            selected[orig_idx] = true;
                        }
                    }
                }
            }
        }
    }
    s.file_selected = selected;
}

/// Read GTK MultiSelection state and update state.rule_selected
pub fn sync_rule_selection_from_gtk(selection: &gtk::MultiSelection, state: &SharedState) {
    use gtk::prelude::SelectionModelExt;
    let n = state.borrow().rules.rules.len();
    let mut selected = vec![false; n];
    let gtk_selection = selection.selection();
    for i in 0..n as u32 {
        if gtk_selection.contains(i) {
            if let Some(s) = selected.get_mut(i as usize) {
                *s = true;
            }
        }
    }
    state.borrow_mut().rule_selected = selected;
}

pub fn sync_outdated(gui_state: &SharedGuiState, state: &SharedState) {
    gui_state.borrow_mut().results_outdated = !state.borrow().rules.updated;
}

#[expect(dead_code)]
pub fn timestamp_to_date(ts: u64) -> String {
    DateTime::from_timestamp(ts as i64, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}
