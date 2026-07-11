use std::collections::HashMap;

use regex::Regex;

use crate::config::{load_custom_rules, load_rules, save_custom_rules, save_rules_to_file};
use crate::connect::sync::{sync_outdated, sync_rules};
use crate::fls;
use crate::localizer::generate_translation_hashmap;
use crate::rule::rules::{MultipleRules, RuleData, RulePlace, RuleType, Rules, SingleRule};
use crate::state::SharedState;
use crate::ui::state_ui::{EditorState, NotebookTab, SharedEditorState, SharedGuiState};

pub fn open_editor(editor_state: &SharedEditorState, gui_state: &SharedGuiState, state: &SharedState, edit_index: Option<i32>) {
    if let Some(idx) = edit_index {
        if idx >= 0 {
            let state_ref = state.borrow();
            if let Some(rule) = state_ref.rules.rules.get(idx as usize).cloned() {
                drop(state_ref);
                load_rule_into_editor(editor_state, &rule);
                state.borrow_mut().edit_index = Some(idx as usize);
            }
        } else {
            state.borrow_mut().edit_index = None;
            reset_editor(editor_state);
        }
    } else {
        state.borrow_mut().edit_index = None;
        reset_editor(editor_state);
    }
    update_example(editor_state, state);
    gui_state.borrow_mut().rule_editor_open = true;
}

pub fn close_editor(gui_state: &SharedGuiState) {
    gui_state.borrow_mut().rule_editor_open = false;
}

pub fn add_or_update_rule(editor_state: &SharedEditorState, store: &gio::ListStore, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState) {
    let single_rule = read_rule_from_editor(editor_state);

    {
        let mut state_mut = state.borrow_mut();
        if let Some(edit_idx) = state_mut.edit_index.take() {
            if let Some(slot) = state_mut.rules.rules.get_mut(edit_idx) {
                *slot = single_rule;
            } else {
                state_mut.rules.add_single_rule(single_rule);
            }
        } else {
            state_mut.rules.add_single_rule(single_rule);
        }
        state_mut.rules.updated = false;
        let new_len = state_mut.rules.rules.len();
        state_mut.rule_selected.resize(new_len, false);
    }
    sync_rules(store, state);
    refresh_outdated_or_recompute(file_store, state, gui_state);
    gui_state.borrow_mut().rule_editor_open = false;
}

pub fn remove_rule(store: &gio::ListStore, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState, idx: i32) {
    let sel = state.borrow().rule_selection.clone();
    if let Some(sel) = &sel {
        crate::connect::sync::sync_rule_selection_from_gtk(sel, state);
    }
    {
        let mut state_mut = state.borrow_mut();
        if idx < 0 {
            let to_remove: Vec<usize> = state_mut
                .rule_selected
                .iter()
                .enumerate()
                .filter_map(|(i, sel)| if *sel { Some(i) } else { None })
                .collect();
            for i in to_remove.iter().rev() {
                if *i < state_mut.rules.rules.len() {
                    state_mut.rules.rules.remove(*i);
                }
                if *i < state_mut.rule_selected.len() {
                    state_mut.rule_selected.remove(*i);
                }
            }
        } else {
            let i = idx as usize;
            if i < state_mut.rules.rules.len() {
                state_mut.rules.rules.remove(i);
            }
            if i < state_mut.rule_selected.len() {
                state_mut.rule_selected.remove(i);
            }
        }
        state_mut.rules.updated = false;
    }
    sync_rules(store, state);
    refresh_outdated_or_recompute(file_store, state, gui_state);
}

pub fn move_rule_up(store: &gio::ListStore, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState) {
    let sel = state.borrow().rule_selection.clone();
    if let Some(sel) = &sel {
        crate::connect::sync::sync_rule_selection_from_gtk(sel, state);
    }
    {
        let mut state_mut = state.borrow_mut();
        let len = state_mut.rules.rules.len();
        for i in 1..len {
            if state_mut.rule_selected.get(i).copied().unwrap_or(false) && !state_mut.rule_selected.get(i - 1).copied().unwrap_or(false) {
                state_mut.rules.rules.swap(i, i - 1);
                state_mut.rule_selected.swap(i, i - 1);
                state_mut.rules.updated = false;
            }
        }
    }
    sync_rules(store, state);
    refresh_outdated_or_recompute(file_store, state, gui_state);
    crate::connect::sync::restore_rule_selection(state);
}

pub fn move_rule_down(store: &gio::ListStore, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState) {
    let sel = state.borrow().rule_selection.clone();
    if let Some(sel) = &sel {
        crate::connect::sync::sync_rule_selection_from_gtk(sel, state);
    }
    {
        let mut state_mut = state.borrow_mut();
        let len = state_mut.rules.rules.len();
        if len == 0 {
            return;
        }
        for i in (0..len - 1).rev() {
            if state_mut.rule_selected.get(i).copied().unwrap_or(false) && !state_mut.rule_selected.get(i + 1).copied().unwrap_or(false) {
                state_mut.rules.rules.swap(i, i + 1);
                state_mut.rule_selected.swap(i, i + 1);
                state_mut.rules.updated = false;
            }
        }
    }
    sync_rules(store, state);
    refresh_outdated_or_recompute(file_store, state, gui_state);
    crate::connect::sync::restore_rule_selection(state);
}

fn format_captures(regex: &Regex, text: &str) -> String {
    match regex.captures(text) {
        Some(caps) => {
            let n = caps.len();
            let header = fls!("label_replace_captures_number", generate_translation_hashmap(vec![("capture_number", n.to_string())]));
            let mut groups = Vec::with_capacity(n);
            for (i, m) in caps.iter().enumerate() {
                let s = m.map_or("", |x| x.as_str());
                groups.push(format!("{i}: {s}"));
            }
            format!("{header} - {}", groups.join(", "))
        }
        None => fls!("label_replace_no_captures"),
    }
}

pub fn update_example(editor_state: &SharedEditorState, state: &SharedState) {
    let es = editor_state.borrow();

    let single_rule = read_rule_from_editor_inner(&es);

    let regex = if single_rule.rule_data.use_regex {
        match Regex::new(&single_rule.rule_data.text_to_find) {
            Ok(r) => {
                drop(es);
                editor_state.borrow_mut().replace_invalid_regex = false;
                Some(r)
            }
            Err(_) => {
                drop(es);
                let mut es_mut = editor_state.borrow_mut();
                es_mut.replace_invalid_regex = true;
                es_mut.replace_captures_text.clear();
                None
            }
        }
    } else {
        drop(es);
        let mut es_mut = editor_state.borrow_mut();
        es_mut.replace_invalid_regex = false;
        es_mut.replace_captures_text.clear();
        None
    };

    let es = editor_state.borrow();
    if let Some(r) = regex.as_ref() {
        let before = es.example_before_text.clone();
        let captures_text = format_captures(r, &before);
        drop(es);
        editor_state.borrow_mut().replace_captures_text = captures_text;
    } else {
        drop(es);
    }

    let mut all_rules = Rules::new();
    all_rules.rules.push(single_rule);

    let es = editor_state.borrow();
    let before = es.example_before_text.clone();
    let text = all_rules.apply_all_rules_to_item(before, 1, 1, (0, 0, 0, "Parent folder"), &[regex]);
    drop(es);
    editor_state.borrow_mut().example_after_text = text;

    refresh_future_names(state);
}

const RULES_UPDATE_LIMIT: usize = 20000;

pub fn refresh_outdated_or_recompute(file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState) {
    let (files_n, rules_n) = {
        let s = state.borrow();
        (s.files.len(), s.rules.rules.len())
    };
    if rules_n == 0 {
        let mut state_mut = state.borrow_mut();
        for file in &mut state_mut.files {
            if file.future_name != file.name {
                file.future_name = file.name.clone();
            }
        }
        state_mut.rules.updated = true;
        drop(state_mut);
        crate::connect::sync::sync_files(file_store, state);
    } else if files_n * rules_n <= RULES_UPDATE_LIMIT {
        refresh_future_names(state);
        state.borrow_mut().rules.updated = true;
        crate::connect::sync::sync_files(file_store, state);
    }
    sync_outdated(gui_state, state);
}

pub fn refresh_future_names(state: &SharedState) {
    let mut state_mut = state.borrow_mut();
    let rules_clone = state_mut.rules.clone();
    let compiled_regexes: Vec<Option<Regex>> = rules_clone
        .rules
        .iter()
        .map(|r| if r.rule_data.use_regex { Regex::new(&r.rule_data.text_to_find).ok() } else { None })
        .collect();

    let mut folder_counter: HashMap<String, u32> = HashMap::new();
    for (idx, file) in state_mut.files.iter_mut().enumerate() {
        let in_folder = folder_counter.entry(file.path.clone()).or_insert(0);
        *in_folder += 1;
        let future = rules_clone.apply_all_rules_to_item(
            file.name.clone(),
            (idx + 1) as u64,
            *in_folder,
            (file.modification_date, file.creation_date, file.size, &file.path),
            &compiled_regexes,
        );
        file.future_name = future;
    }
}

pub fn read_rule_from_editor(editor_state: &SharedEditorState) -> SingleRule {
    let es = editor_state.borrow();
    read_rule_from_editor_inner(&es)
}

fn read_rule_from_editor_inner(es: &EditorState) -> SingleRule {
    let mut rule_data = RuleData::new();
    let (rule_type, rule_place, rule_description) = match es.current_tab {
        NotebookTab::Custom => {
            rule_data.custom_text = es.custom_text.clone();
            let desc = fls!(
                "rule_description_custom_rule",
                generate_translation_hashmap(vec![("custom_rule", rule_data.custom_text.clone())])
            );
            (RuleType::Custom, RulePlace::None, desc)
        }
        NotebookTab::CaseSize => {
            rule_data.to_lowercase = es.case_lowercase;
            let place = es.case_place;
            let desc = if rule_data.to_lowercase {
                format!("{} {}", fls!("rule_description_lowercase"), fls!("rule_description_text"))
            } else {
                format!("{} {}", fls!("rule_description_uppercase"), fls!("rule_description_text"))
            };
            (RuleType::CaseSize, place, desc)
        }
        NotebookTab::Purge => {
            let place = es.purge_place;
            (RuleType::Purge, place, String::new())
        }
        NotebookTab::AddNumber => {
            let place = es.add_number_place;
            rule_data.number_start = es.add_number_start.parse::<i64>().unwrap_or(0);
            rule_data.number_step = es.add_number_step.parse::<i64>().unwrap_or(1);
            rule_data.fill_with_zeros = es.add_number_zeros.parse::<i64>().unwrap_or(0);
            let zeros = if rule_data.fill_with_zeros > 0 {
                format!(
                    " {}",
                    fls!(
                        "rule_description_zeros",
                        generate_translation_hashmap(vec![("zeros", rule_data.fill_with_zeros.to_string())])
                    )
                )
            } else {
                String::new()
            };
            let desc = fls!(
                "rule_description_step",
                generate_translation_hashmap(vec![
                    ("step", rule_data.number_step.to_string()),
                    ("start", rule_data.number_start.to_string()),
                    ("zeros", zeros),
                ])
            );
            (RuleType::AddNumber, place, desc)
        }
        NotebookTab::AddText => {
            let place = es.add_text_place;
            rule_data.add_text_text = es.add_text_text.clone();
            let desc = format!("{} {}", fls!("rule_description_added_text"), rule_data.add_text_text);
            (RuleType::AddText, place, desc)
        }
        NotebookTab::Replace => {
            rule_data.case_sensitive = es.replace_case_sensitive;
            rule_data.use_regex = es.replace_use_regex;
            rule_data.regex_replace_all = es.replace_all_occurrences;
            rule_data.text_to_find = es.replace_text_to_find.clone();
            rule_data.text_to_replace = es.replace_text_to_replace.clone();
            let place = if rule_data.use_regex { RulePlace::None } else { es.replace_place };
            let additional_regex_text = if rule_data.use_regex { " regex" } else { "" };
            let desc = fls!(
                "rule_description_replace",
                generate_translation_hashmap(vec![
                    ("text_to_find", rule_data.text_to_find.clone()),
                    ("text_to_replace", rule_data.text_to_replace.clone()),
                    ("additional_regex_text", additional_regex_text.to_string()),
                ])
            );
            (RuleType::Replace, place, desc)
        }
        NotebookTab::Trim => {
            rule_data.case_sensitive = es.trim_case_sensitive;
            rule_data.trim_text = es.trim_text.clone();
            let place = es.trim_place;
            let where_remove = match place {
                RulePlace::FromNameStart => fls!("rule_description_start"),
                RulePlace::FromNameEndReverse => fls!("rule_description_end_of_name"),
                RulePlace::FromExtensionStart => fls!("rule_description_extension"),
                RulePlace::FromExtensionEndReverse => fls!("rule_description_end_of_extension"),
                _ => String::new(),
            };
            let desc = fls!(
                "rule_description_trimming",
                generate_translation_hashmap(vec![("trim_text", rule_data.trim_text.clone()), ("where_remove", where_remove)])
            );
            (RuleType::Trim, place, desc)
        }
        NotebookTab::Normalize => {
            rule_data.full_normalize = es.normalize_full;
            let desc = if rule_data.full_normalize {
                fls!("rule_description_full_normalize")
            } else {
                fls!("rule_description_partial_normalize")
            };
            (RuleType::Normalize, RulePlace::ExtensionAndName, desc)
        }
    };

    SingleRule {
        rule_type,
        rule_place,
        rule_data,
        rule_description,
    }
}

pub fn load_rule_into_editor(editor_state: &SharedEditorState, rule: &SingleRule) {
    let mut es = editor_state.borrow_mut();
    let tab = match rule.rule_type {
        RuleType::Custom => NotebookTab::Custom,
        RuleType::CaseSize => NotebookTab::CaseSize,
        RuleType::Purge => NotebookTab::Purge,
        RuleType::AddNumber => NotebookTab::AddNumber,
        RuleType::AddText => NotebookTab::AddText,
        RuleType::Replace => NotebookTab::Replace,
        RuleType::Trim => NotebookTab::Trim,
        RuleType::Normalize => NotebookTab::Normalize,
    };
    es.current_tab = tab;

    match rule.rule_type {
        RuleType::Custom => {
            es.custom_text = rule.rule_data.custom_text.clone();
        }
        RuleType::CaseSize => {
            es.case_lowercase = rule.rule_data.to_lowercase;
            es.case_place = rule.rule_place;
        }
        RuleType::Purge => {
            es.purge_place = rule.rule_place;
        }
        RuleType::AddNumber => {
            es.add_number_place = rule.rule_place;
            es.add_number_start = rule.rule_data.number_start.to_string();
            es.add_number_step = rule.rule_data.number_step.to_string();
            es.add_number_zeros = rule.rule_data.fill_with_zeros.to_string();
        }
        RuleType::AddText => {
            es.add_text_place = rule.rule_place;
            es.add_text_text = rule.rule_data.add_text_text.clone();
        }
        RuleType::Replace => {
            es.replace_place = rule.rule_place;
            es.replace_case_sensitive = rule.rule_data.case_sensitive;
            es.replace_use_regex = rule.rule_data.use_regex;
            es.replace_all_occurrences = rule.rule_data.regex_replace_all;
            es.replace_text_to_find = rule.rule_data.text_to_find.clone();
            es.replace_text_to_replace = rule.rule_data.text_to_replace.clone();
        }
        RuleType::Trim => {
            es.trim_place = rule.rule_place;
            es.trim_case_sensitive = rule.rule_data.case_sensitive;
            es.trim_text = rule.rule_data.trim_text.clone();
        }
        RuleType::Normalize => {
            es.normalize_full = rule.rule_data.full_normalize;
        }
    }
}

pub fn reset_editor(editor_state: &SharedEditorState) {
    let mut es = editor_state.borrow_mut();
    *es = EditorState::default();
}

pub fn save_rule_set(state: &SharedState, name: &str) {
    if name.trim().is_empty() {
        return;
    }
    let rules = state.borrow().rules.rules.clone();
    let new_entry = MultipleRules { name: name.to_string(), rules };
    let mut all = load_rules();
    if let Some(existing) = all.iter_mut().find(|m| m.name == name) {
        *existing = new_entry;
    } else {
        all.push(new_entry);
    }
    save_rules_to_file(&all);
}

pub fn load_rule_set(store: &gio::ListStore, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState, index: i32) {
    if index < 0 {
        return;
    }
    let all = load_rules();
    if let Some(entry) = all.get(index as usize) {
        {
            let mut state_mut = state.borrow_mut();
            state_mut.rules.rules = entry.rules.clone();
            state_mut.rules.updated = false;
            let len = state_mut.rules.rules.len();
            state_mut.rule_selected.clear();
            state_mut.rule_selected.resize(len, false);
        }
        sync_rules(store, state);
        refresh_outdated_or_recompute(file_store, state, gui_state);
    }
}

pub fn delete_rule_set(index: i32) {
    if index < 0 {
        return;
    }
    let mut all = load_rules();
    if (index as usize) < all.len() {
        all.remove(index as usize);
        save_rules_to_file(&all);
    }
}

pub fn refresh_rule_sets(gui_state: &SharedGuiState) {
    let all = load_rules();
    let names: Vec<String> = all.iter().map(|m| m.name.clone()).collect();
    let names_text = if names.is_empty() {
        String::new()
    } else {
        fls!("edit_names_used_in_rules", generate_translation_hashmap(vec![("rules", names.join(", "))]))
    };
    gui_state.borrow_mut().existing_rule_set_names = names_text;
}

pub fn refresh_custom_texts() -> Vec<String> {
    load_custom_rules()
}

pub fn save_custom_text(text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let mut current = load_custom_rules();
    if !current.iter().any(|t| t == text) {
        current.push(text.to_string());
        save_custom_rules(&current);
    }
}

pub fn load_custom_text_into_editor(editor_state: &SharedEditorState, index: i32) {
    if index < 0 {
        return;
    }
    let all = load_custom_rules();
    if let Some(text) = all.get(index as usize) {
        editor_state.borrow_mut().custom_text = text.clone();
    }
}

pub fn delete_custom_text(index: i32) {
    if index < 0 {
        return;
    }
    let mut all = load_custom_rules();
    if (index as usize) < all.len() {
        all.remove(index as usize);
        save_custom_rules(&all);
    }
}
