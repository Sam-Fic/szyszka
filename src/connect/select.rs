use crate::connect::sync::restore_file_selection;
use crate::files::{regex_check, CHARACTER};
use crate::state::SharedState;
use crate::ui::state_ui::SelectMode;

pub fn apply_select(_store: &gio::ListStore, state: &SharedState, mode: SelectMode) {
    {
        let mut state_mut = state.borrow_mut();
        let len = state_mut.files.len();
        state_mut.file_selected.resize(len, false);
        match mode {
            SelectMode::SelectAll => {
                for s in &mut state_mut.file_selected {
                    *s = true;
                }
            }
            SelectMode::UnselectAll => {
                for s in &mut state_mut.file_selected {
                    *s = false;
                }
            }
            SelectMode::Reverse => {
                for s in &mut state_mut.file_selected {
                    *s = !*s;
                }
            }
            SelectMode::SelectChanged => {
                let changed: Vec<usize> = state_mut
                    .files
                    .iter()
                    .enumerate()
                    .filter_map(|(i, f)| if f.future_name != f.name { Some(i) } else { None })
                    .collect();
                for idx in changed {
                    if let Some(s) = state_mut.file_selected.get_mut(idx) {
                        *s = true;
                    }
                }
            }
            SelectMode::UnselectChanged => {
                let changed: Vec<usize> = state_mut
                    .files
                    .iter()
                    .enumerate()
                    .filter_map(|(i, f)| if f.future_name != f.name { Some(i) } else { None })
                    .collect();
                for idx in changed {
                    if let Some(s) = state_mut.file_selected.get_mut(idx) {
                        *s = false;
                    }
                }
            }
        }
    }
    restore_file_selection(state);
}

pub fn apply_select_custom(_store: &gio::ListStore, state: &SharedState, pattern: &str, include_dirs: bool, mode_index: i32, select: bool) {
    {
        let mut state_mut = state.borrow_mut();
        let snapshot: Vec<(String, String, String, bool)> = state_mut.files.iter().map(|f| (f.path.clone(), f.name.clone(), f.future_name.clone(), f.is_dir)).collect();

        let len = state_mut.files.len();
        state_mut.file_selected.resize(len, false);

        for (idx, (path, current_name, future_name, is_dir)) in snapshot.iter().enumerate() {
            let matched = match mode_index {
                5 => *is_dir == include_dirs,
                _ => {
                    if *is_dir && !include_dirs {
                        false
                    } else {
                        let target = match mode_index {
                            1 => current_name.clone(),
                            2 => future_name.clone(),
                            3 => format!("{path}{CHARACTER}{current_name}"),
                            4 => format!("{path}{CHARACTER}{future_name}"),
                            _ => path.clone(),
                        };
                        regex_check(pattern, &target)
                    }
                }
            };
            if matched {
                if let Some(s) = state_mut.file_selected.get_mut(idx) {
                    *s = select;
                }
            }
        }
    }
    restore_file_selection(state);
}
