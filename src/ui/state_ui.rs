use std::cell::RefCell;
use std::rc::Rc;

use crate::rule::rules::RulePlace;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotebookTab {
    Custom,
    CaseSize,
    Purge,
    AddNumber,
    AddText,
    Replace,
    Trim,
    Normalize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectMode {
    SelectAll,
    UnselectAll,
    Reverse,
    SelectChanged,
    UnselectChanged,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[expect(dead_code)]
pub enum SortColumn {
    None,
    TypeC,
    Current,
    Future,
    Path,
}

pub struct EditorState {
    pub current_tab: NotebookTab,
    pub example_before_text: String,
    pub example_after_text: String,
    pub custom_text: String,
    pub case_lowercase: bool,
    pub case_place: RulePlace,
    pub purge_place: RulePlace,
    pub add_number_place: RulePlace,
    pub add_number_start: String,
    pub add_number_step: String,
    pub add_number_zeros: String,
    pub add_text_place: RulePlace,
    pub add_text_text: String,
    pub replace_place: RulePlace,
    pub replace_case_sensitive: bool,
    pub replace_use_regex: bool,
    pub replace_all_occurrences: bool,
    pub replace_text_to_find: String,
    pub replace_text_to_replace: String,
    pub replace_invalid_regex: bool,
    pub replace_captures_text: String,
    pub trim_place: RulePlace,
    pub trim_case_sensitive: bool,
    pub trim_text: String,
    pub normalize_full: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            current_tab: NotebookTab::Custom,
            example_before_text: "Gżegżółka.Txt".to_string(),
            example_after_text: "Gżegżółka.Txt".to_string(),
            custom_text: "FILE_$(N).$(EXT)".to_string(),
            case_lowercase: true,
            case_place: RulePlace::Name,
            purge_place: RulePlace::Name,
            add_number_place: RulePlace::BeforeName,
            add_number_start: "0".to_string(),
            add_number_step: "1".to_string(),
            add_number_zeros: "0".to_string(),
            add_text_place: RulePlace::BeforeName,
            add_text_text: String::new(),
            replace_place: RulePlace::Name,
            replace_case_sensitive: false,
            replace_use_regex: false,
            replace_all_occurrences: true,
            replace_text_to_find: String::new(),
            replace_text_to_replace: String::new(),
            replace_invalid_regex: false,
            replace_captures_text: String::new(),
            trim_place: RulePlace::FromNameStart,
            trim_case_sensitive: false,
            trim_text: String::new(),
            normalize_full: true,
        }
    }
}

#[expect(dead_code)]
pub struct GuiState {
    pub rule_editor_open: bool,
    pub settings_open: bool,
    pub select_popup_open: bool,
    pub load_popup_open: bool,
    pub confirm_dialog_open: bool,
    pub outdated_warning_open: bool,
    pub results_dialog_open: bool,
    pub message_dialog_open: bool,
    pub select_custom_dialog_open: bool,
    pub save_rule_dialog_open: bool,
    pub add_folders_dialog_open: bool,

    pub message_dialog_title: String,
    pub message_dialog_text: String,

    pub file_count: i32,
    pub rule_count: i32,
    pub results_outdated: bool,

    pub properly_renamed: i32,
    pub ignored_count: i32,
    pub failed_text: String,
    pub failed_total: i32,
    pub failed_page: i32,
    pub failed_pages: i32,

    pub select_custom_text: String,
    pub select_custom_select_dirs: bool,
    pub select_custom_mode_index: i32,

    pub save_rule_name: String,
    pub existing_rule_set_names: String,

    pub add_folder_scan_inside: bool,
    pub add_folder_ignore_folders: bool,
    pub add_folder_picked_paths: Vec<String>,

    pub sort_column: SortColumn,
    pub sort_descending: bool,

    pub last_file_anchor_idx: i32,
    pub last_rule_anchor_idx: i32,

    pub progress_active: bool,
    pub progress_title: String,
    pub progress_message: String,
    pub progress_current: usize,
    pub progress_total: usize,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            rule_editor_open: false,
            settings_open: false,
            select_popup_open: false,
            load_popup_open: false,
            confirm_dialog_open: false,
            outdated_warning_open: false,
            results_dialog_open: false,
            message_dialog_open: false,
            select_custom_dialog_open: false,
            save_rule_dialog_open: false,
            add_folders_dialog_open: false,

            message_dialog_title: String::new(),
            message_dialog_text: String::new(),

            file_count: 0,
            rule_count: 0,
            results_outdated: false,

            properly_renamed: 0,
            ignored_count: 0,
            failed_text: String::new(),
            failed_total: 0,
            failed_page: 0,
            failed_pages: 0,

            select_custom_text: String::new(),
            select_custom_select_dirs: true,
            select_custom_mode_index: 0,

            save_rule_name: String::new(),
            existing_rule_set_names: String::new(),

            add_folder_scan_inside: false,
            add_folder_ignore_folders: false,
            add_folder_picked_paths: Vec::new(),

            sort_column: SortColumn::None,
            sort_descending: false,

            last_file_anchor_idx: -1,
            last_rule_anchor_idx: -1,

            progress_active: false,
            progress_title: String::new(),
            progress_message: String::new(),
            progress_current: 0,
            progress_total: 0,
        }
    }
}

pub type SharedEditorState = Rc<RefCell<EditorState>>;
pub type SharedGuiState = Rc<RefCell<GuiState>>;

pub fn new_shared_editor_state() -> SharedEditorState {
    Rc::new(RefCell::new(EditorState::default()))
}

pub fn new_shared_gui_state() -> SharedGuiState {
    Rc::new(RefCell::new(GuiState::default()))
}
