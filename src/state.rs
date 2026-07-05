use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::files::{ItemStruct, ResultEntries};
use crate::rule::rules::Rules;

#[derive(Default)]
pub struct AppState {
    pub files: Vec<ItemStruct>,
    pub file_selected: Vec<bool>,
    pub rules: Rules,
    pub rule_selected: Vec<bool>,
    pub result_entries: ResultEntries,
    pub edit_index: Option<usize>,
    pub async_active: bool,
    pub pending_folders: Vec<PathBuf>,
    pub failed_renames: Vec<String>,
    pub file_selection: Option<gtk::MultiSelection>,
    pub file_sort_model: Option<gtk::SortListModel>,
    pub rule_selection: Option<gtk::MultiSelection>,
}

pub type SharedState = Rc<RefCell<AppState>>;

pub fn new_shared() -> SharedState {
    Rc::new(RefCell::new(AppState::default()))
}
