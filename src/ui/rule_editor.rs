use adw::prelude::*;
use gtk::prelude::*;

use super::state_ui::{NotebookTab, SharedEditorState, SharedGuiState};
use crate::rule::rules::RulePlace;
use crate::state::SharedState;

fn icon_button(label: &str, icon: &str) -> gtk::Button {
    let btn = gtk::Button::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_halign(gtk::Align::Center);
    content.append(&gtk::Image::from_icon_name(icon));
    content.append(&gtk::Label::new(Some(label)));
    btn.set_child(Some(&content));
    btn
}

fn combo_row(
    title: &str,
    options: &[&str],
    active: usize,
    es: &SharedEditorState,
    after_label: &gtk::Label,
    set_val: impl Fn(&mut crate::ui::state_ui::EditorState, u32) + 'static,
) -> adw::ComboRow {
    let combo = adw::ComboRow::builder().title(title).model(&gtk::StringList::new(options)).selected(active as u32).build();
    let es = es.clone();
    let lbl = after_label.clone();
    combo.connect_selected_notify(move |c| {
        set_val(&mut es.borrow_mut(), c.selected());
        crate::connect::rules_ops::update_example(&es, &SharedState::default());
        lbl.set_text(&es.borrow().example_after_text);
    });
    combo
}

fn entry_row(
    title: &str,
    text: &str,
    es: &SharedEditorState,
    after_label: &gtk::Label,
    set_val: impl Fn(&mut crate::ui::state_ui::EditorState, String) + 'static,
) -> adw::EntryRow {
    let row = adw::EntryRow::builder().title(title).text(text).build();
    let es = es.clone();
    let lbl = after_label.clone();
    row.connect_changed(move |e| {
        set_val(&mut es.borrow_mut(), e.text().to_string());
        crate::connect::rules_ops::update_example(&es, &SharedState::default());
        lbl.set_text(&es.borrow().example_after_text);
    });
    row
}

fn switch_row(
    title: &str,
    active: bool,
    es: &SharedEditorState,
    after_label: &gtk::Label,
    set_val: impl Fn(&mut crate::ui::state_ui::EditorState, bool) + 'static,
) -> adw::SwitchRow {
    let row = adw::SwitchRow::builder().title(title).active(active).build();
    let es = es.clone();
    let lbl = after_label.clone();
    row.connect_active_notify(move |r| {
        set_val(&mut es.borrow_mut(), r.is_active());
        crate::connect::rules_ops::update_example(&es, &SharedState::default());
        lbl.set_text(&es.borrow().example_after_text);
    });
    row
}

pub fn show_rule_editor(
    window: &adw::ApplicationWindow,
    editor_state: &SharedEditorState,
    state: &SharedState,
    rule_store: &gio::ListStore,
    file_store: &gio::ListStore,
    gui_state: &SharedGuiState,
    edit_index: Option<i32>,
) {
    crate::connect::rules_ops::open_editor(editor_state, gui_state, state, edit_index);

    let editor_dialog = adw::Dialog::builder()
        .title(&crate::fls!("rule_editor_title"))
        .content_width(800)
        .content_height(640)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);

    let header = gtk::HeaderBar::new();
    header.set_show_title_buttons(false);
    header.set_title_widget(Some(&gtk::Label::new(Some(&crate::fls!("rule_editor_title")))));
    let cancel_btn = gtk::Button::with_label(&crate::fls!("rule_editor_cancel"));
    let add_btn = gtk::Button::with_label(&crate::fls!("rule_editor_add"));
    add_btn.add_css_class("suggested-action");
    header.pack_start(&cancel_btn);
    header.pack_end(&add_btn);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.set_vexpand(true);

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_vexpand(true);

    // GNOME standard sidebar
    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_width_request(180);
    sidebar.set_margin_top(8);
    sidebar.set_margin_bottom(8);
    sidebar.set_margin_start(8);

    content.append(&sidebar);

    let after_label = gtk::Label::builder().hexpand(true).xalign(0.0).build();
    {
        let es = editor_state.borrow();
        after_label.set_text(&es.example_after_text);
    }

    let after_label_ref = after_label.clone();

    // Custom tab
    {
        let page = adw::PreferencesPage::new();

        let desc_group = adw::PreferencesGroup::new();
        let instruction = gtk::Label::builder()
            .label(&crate::fls!("label_custom_instruction"))
            .xalign(0.0)
            .wrap(true)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        instruction.add_css_class("dim-label");
        desc_group.add(&instruction);
        page.add(&desc_group);

        let entry_group = adw::PreferencesGroup::builder().title("Template").build();
        let entry = gtk::Entry::builder()
            .text(&editor_state.borrow().custom_text)
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();
        let suppress_change = std::rc::Rc::new(std::cell::Cell::new(false));
        {
            let es = editor_state.clone();
            let lbl = after_label.clone();
            let sc = suppress_change.clone();
            entry.connect_changed(move |e| {
                if sc.get() {
                    return;
                }
                es.borrow_mut().custom_text = e.text().to_string();
                crate::connect::rules_ops::update_example(&es, &SharedState::default());
                lbl.set_text(&es.borrow().example_after_text);
            });
        }
        entry_group.add(&entry);
        page.add(&entry_group);

        let saved_group = adw::PreferencesGroup::builder().title(&crate::fls!("rule_editor_custom_saved")).build();
        let saved_list_box = gtk::ListBox::new();
        saved_list_box.add_css_class("boxed-list");

        let refresh_saved_list = {
            let saved_list_box = saved_list_box.clone();
            let es = editor_state.clone();
            let entry = entry.clone();
            let lbl = after_label_ref.clone();
            move || {
                while let Some(child) = saved_list_box.first_child() {
                    saved_list_box.remove(&child);
                }
                for (idx, text) in crate::connect::rules_ops::refresh_custom_texts().iter().enumerate() {
                    let row = adw::ActionRow::builder().title(text).activatable(false).build();
                    let load_btn = gtk::Button::from_icon_name("document-open-symbolic");
                    load_btn.set_size_request(32, 32);
                    load_btn.set_vexpand(false);
                    load_btn.set_valign(gtk::Align::Center);
                    load_btn.add_css_class("flat");
                    load_btn.set_tooltip_text(Some(&crate::fls!("rule_editor_load")));
                    let es2 = es.clone();
                    let entry2 = entry.clone();
                    let lbl2 = lbl.clone();
                    let sc = suppress_change.clone();
                    let i = idx as i32;
                    load_btn.connect_clicked(move |_| {
                        crate::connect::rules_ops::load_custom_text_into_editor(&es2, i);
                        let text = es2.borrow().custom_text.clone();
                        sc.set(true);
                        entry2.set_text(&text);
                        sc.set(false);
                        crate::connect::rules_ops::update_example(&es2, &SharedState::default());
                        let after = es2.borrow().example_after_text.clone();
                        lbl2.set_text(&after);
                    });
                    row.add_suffix(&load_btn);
                    let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                    del_btn.set_size_request(32, 32);
                    del_btn.set_vexpand(false);
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.add_css_class("flat");
                    del_btn.add_css_class("destructive-action");
                    del_btn.set_tooltip_text(Some(&crate::fls!("rule_editor_delete")));
                    let i = idx as i32;
                    del_btn.connect_clicked(move |_| crate::connect::rules_ops::delete_custom_text(i));
                    row.add_suffix(&del_btn);
                    saved_list_box.append(&row);
                }
            }
        };
        refresh_saved_list();
        saved_group.add(&saved_list_box);
        page.add(&saved_group);

        // Save button (after list is set up so it can refresh)
        let action_group = adw::PreferencesGroup::new();
        let save_btn = icon_button(&crate::fls!("rule_editor_custom_save"), "document-save-symbolic");
        save_btn.set_halign(gtk::Align::Start);
        {
            let entry = entry.clone();
            let refresh = refresh_saved_list.clone();
            save_btn.connect_clicked(move |_| {
                crate::connect::rules_ops::save_custom_text(&entry.text());
                refresh();
            });
        }
        action_group.add(&save_btn);
        page.add(&action_group);
        stack.add_titled(&page, Some("custom"), &crate::fls!("tab_custom"));
        stack.page(&page).set_icon_name("document-edit-symbolic");
    }

    // Case/Size tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::new();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("rule_editor_tool_type"),
            &[&crate::fls!("ctrl_lowercase"), &crate::fls!("ctrl_uppercase")],
            if es.case_lowercase { 0 } else { 1 },
            editor_state,
            &after_label,
            |es, v| es.case_lowercase = v == 0,
        ));
        group.add(&combo_row(
            &crate::fls!("rule_editor_usage_type"),
            &[&crate::fls!("ctrl_only_name"), &crate::fls!("ctrl_only_extension"), &crate::fls!("ctrl_both")],
            match es.case_place {
                RulePlace::Name => 0,
                RulePlace::Extension => 1,
                _ => 2,
            },
            editor_state,
            &after_label,
            |es, v| {
                es.case_place = match v {
                    0 => RulePlace::Name,
                    1 => RulePlace::Extension,
                    _ => RulePlace::ExtensionAndName,
                }
            },
        ));
        drop(es);
        page.add(&group);
        stack.add_titled(&page, Some("case"), &crate::fls!("tab_case_size"));
        stack.page(&page).set_icon_name("format-text-symbolic");
    }

    // Purge tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::new();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("rule_editor_usage_type"),
            &[&crate::fls!("ctrl_only_name"), &crate::fls!("ctrl_only_extension"), &crate::fls!("ctrl_both")],
            match es.purge_place {
                RulePlace::Name => 0,
                RulePlace::Extension => 1,
                _ => 2,
            },
            editor_state,
            &after_label,
            |es, v| {
                es.purge_place = match v {
                    0 => RulePlace::Name,
                    1 => RulePlace::Extension,
                    _ => RulePlace::ExtensionAndName,
                }
            },
        ));
        drop(es);
        page.add(&group);
        stack.add_titled(&page, Some("purge"), &crate::fls!("tab_purge"));
        stack.page(&page).set_icon_name("edit-clear-all-symbolic");
    }

    // Add Number tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::builder().title(&crate::fls!("label_add_number_place")).build();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("label_add_number_place"),
            &[&crate::fls!("ctrl_before_name"), &crate::fls!("ctrl_after_name")],
            match es.add_number_place {
                RulePlace::BeforeName => 0,
                _ => 1,
            },
            editor_state,
            &after_label,
            |es, v| es.add_number_place = if v == 0 { RulePlace::BeforeName } else { RulePlace::AfterName },
        ));
        let settings_group = adw::PreferencesGroup::builder().title(&crate::fls!("label_add_number_settings")).build();
        settings_group.add(&entry_row(&crate::fls!("ctrl_start_number"), &es.add_number_start, editor_state, &after_label, |es, v| {
            es.add_number_start = v
        }));
        settings_group.add(&entry_row(&crate::fls!("ctrl_step"), &es.add_number_step, editor_state, &after_label, |es, v| {
            es.add_number_step = v
        }));
        settings_group.add(&entry_row(&crate::fls!("ctrl_fill_zeros"), &es.add_number_zeros, editor_state, &after_label, |es, v| {
            es.add_number_zeros = v
        }));
        drop(es);
        page.add(&group);
        page.add(&settings_group);
        stack.add_titled(&page, Some("add_number"), &crate::fls!("tab_add_number"));
        stack.page(&page).set_icon_name("format-number-symbolic");
    }

    // Add Text tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::new();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("rule_editor_usage_type"),
            &[&crate::fls!("ctrl_before_name"), &crate::fls!("ctrl_after_name")],
            match es.add_text_place {
                RulePlace::BeforeName => 0,
                _ => 1,
            },
            editor_state,
            &after_label,
            |es, v| es.add_text_place = if v == 0 { RulePlace::BeforeName } else { RulePlace::AfterName },
        ));
        group.add(&entry_row(&crate::fls!("label_add_text"), &es.add_text_text, editor_state, &after_label, |es, v| {
            es.add_text_text = v
        }));
        drop(es);
        page.add(&group);
        stack.add_titled(&page, Some("add_text"), &crate::fls!("tab_add_text"));
        stack.page(&page).set_icon_name("document-new-symbolic");
    }

    // Replace tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::builder().title(&crate::fls!("tab_replace")).build();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("ctrl_match_against"),
            &[&crate::fls!("ctrl_only_name"), &crate::fls!("ctrl_only_extension"), &crate::fls!("ctrl_both")],
            match es.replace_place {
                RulePlace::Name => 0,
                RulePlace::Extension => 1,
                _ => 2,
            },
            editor_state,
            &after_label,
            |es, v| {
                es.replace_place = match v {
                    0 => RulePlace::Name,
                    1 => RulePlace::Extension,
                    _ => RulePlace::ExtensionAndName,
                }
            },
        ));
        group.add(&combo_row(
            &crate::fls!("label_usage_type"),
            &[&crate::fls!("ctrl_case_sensitive"), &crate::fls!("ctrl_case_insensitive")],
            if es.replace_case_sensitive { 0 } else { 1 },
            editor_state,
            &after_label,
            |es, v| es.replace_case_sensitive = v == 0,
        ));
        group.add(&switch_row(&crate::fls!("ctrl_use_regex"), es.replace_use_regex, editor_state, &after_label, |es, v| {
            es.replace_use_regex = v
        }));
        group.add(&switch_row(
            &crate::fls!("ctrl_replace_all"),
            es.replace_all_occurrences,
            editor_state,
            &after_label,
            |es, v| es.replace_all_occurrences = v,
        ));
        group.add(&entry_row(
            &crate::fls!("ctrl_text_to_find"),
            &es.replace_text_to_find,
            editor_state,
            &after_label,
            |es, v| es.replace_text_to_find = v,
        ));
        group.add(&entry_row(
            &crate::fls!("ctrl_text_to_replace"),
            &es.replace_text_to_replace,
            editor_state,
            &after_label,
            |es, v| es.replace_text_to_replace = v,
        ));
        drop(es);
        page.add(&group);
        stack.add_titled(&page, Some("replace"), &crate::fls!("tab_replace"));
        stack.page(&page).set_icon_name("edit-find-replace-symbolic");
    }

    // Trim tab
    {
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::builder().title(&crate::fls!("tab_trim")).build();
        let es = editor_state.borrow();
        group.add(&combo_row(
            &crate::fls!("ctrl_match_against"),
            &[
                &crate::fls!("ctrl_name_start"),
                &crate::fls!("ctrl_name_end"),
                &crate::fls!("ctrl_extension_start"),
                &crate::fls!("ctrl_extension_end"),
            ],
            match es.trim_place {
                RulePlace::FromNameStart => 0,
                RulePlace::FromNameEndReverse => 1,
                RulePlace::FromExtensionStart => 2,
                _ => 3,
            },
            editor_state,
            &after_label,
            |es, v| {
                es.trim_place = match v {
                    0 => RulePlace::FromNameStart,
                    1 => RulePlace::FromNameEndReverse,
                    2 => RulePlace::FromExtensionStart,
                    _ => RulePlace::FromExtensionEndReverse,
                }
            },
        ));
        group.add(&combo_row(
            &crate::fls!("label_usage_type"),
            &[&crate::fls!("ctrl_case_sensitive"), &crate::fls!("ctrl_case_insensitive")],
            if es.trim_case_sensitive { 0 } else { 1 },
            editor_state,
            &after_label,
            |es, v| es.trim_case_sensitive = v == 0,
        ));
        group.add(&entry_row(&crate::fls!("ctrl_trim_text"), &es.trim_text, editor_state, &after_label, |es, v| {
            es.trim_text = v
        }));
        drop(es);
        page.add(&group);
        stack.add_titled(&page, Some("trim"), &crate::fls!("tab_trim"));
        stack.page(&page).set_icon_name("format-cut-symbolic");
    }

    // Normalize tab
    {
        let page = adw::PreferencesPage::new();

        let desc_group = adw::PreferencesGroup::new();
        let desc = gtk::Label::builder()
            .label(&crate::fls!("label_normalize_name"))
            .xalign(0.0)
            .wrap(true)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        desc.add_css_class("dim-label");
        desc_group.add(&desc);
        page.add(&desc_group);

        let options_group = adw::PreferencesGroup::new();
        options_group.add(&combo_row(
            &crate::fls!("rule_editor_usage_type"),
            &[&crate::fls!("ctrl_everything"), &crate::fls!("ctrl_partial")],
            if editor_state.borrow().normalize_full { 0 } else { 1 },
            editor_state,
            &after_label,
            |es, v| es.normalize_full = v == 0,
        ));
        page.add(&options_group);

        stack.add_titled(&page, Some("normalize"), &crate::fls!("tab_normalize"));
        stack.page(&page).set_icon_name("media-playlist-shuffle-symbolic");
    }

    stack.set_visible_child_name(match editor_state.borrow().current_tab {
        NotebookTab::Custom => "custom",
        NotebookTab::CaseSize => "case",
        NotebookTab::Purge => "purge",
        NotebookTab::AddNumber => "add_number",
        NotebookTab::AddText => "add_text",
        NotebookTab::Replace => "replace",
        NotebookTab::Trim => "trim",
        NotebookTab::Normalize => "normalize",
    });

    // Sync current_tab when user switches sidebar tabs.
    {
        let es = editor_state.clone();
        stack.connect_notify_local(Some("visible-child"), move |stack, _| {
            let name = stack.visible_child_name().map(|n| n.as_str().to_owned()).unwrap_or_default();
            let tab = match name.as_str() {
                "custom" => NotebookTab::Custom,
                "case" => NotebookTab::CaseSize,
                "purge" => NotebookTab::Purge,
                "add_number" => NotebookTab::AddNumber,
                "add_text" => NotebookTab::AddText,
                "replace" => NotebookTab::Replace,
                "trim" => NotebookTab::Trim,
                "normalize" => NotebookTab::Normalize,
                _ => return,
            };
            es.borrow_mut().current_tab = tab;
        });
    }

    // Example panel - use adw::PreferencesGroup for native card style
    let example_group = adw::PreferencesGroup::builder().title(&crate::fls!("rule_editor_example")).build();

    let before_entry = adw::EntryRow::builder()
        .title(&crate::fls!("rule_editor_example_before"))
        .text(&editor_state.borrow().example_before_text)
        .show_apply_button(false)
        .build();

    let reset_btn = gtk::Button::from_icon_name("edit-undo-symbolic");
    reset_btn.set_size_request(32, 32);
    reset_btn.set_vexpand(false);
    reset_btn.set_valign(gtk::Align::Center);
    reset_btn.add_css_class("flat");
    reset_btn.set_tooltip_text(Some(&crate::fls!("rule_editor_reset")));
    {
        let be = before_entry.clone();
        let es = editor_state.clone();
        let lbl = after_label.clone();
        reset_btn.connect_clicked(move |_| {
            be.set_text("Gżegżółka.Txt");
            es.borrow_mut().example_before_text = "Gżegżółka.Txt".to_string();
            crate::connect::rules_ops::update_example(&es, &SharedState::default());
            lbl.set_text(&es.borrow().example_after_text);
        });
    }
    before_entry.add_suffix(&reset_btn);
    example_group.add(&before_entry);

    let after_row = adw::ActionRow::builder().title(&crate::fls!("rule_editor_example_after")).activatable(false).build();
    after_label.set_halign(gtk::Align::End);
    after_label.set_hexpand(true);
    after_label.add_css_class("heading");
    after_row.add_suffix(&after_label);
    example_group.add(&after_row);

    // Wire before_entry -> example
    {
        let es = editor_state.clone();
        let lbl = after_label.clone();
        before_entry.connect_changed(move |e| {
            es.borrow_mut().example_before_text = e.text().to_string();
            crate::connect::rules_ops::update_example(&es, &SharedState::default());
            lbl.set_text(&es.borrow().example_after_text);
        });
    }

    // Put example_group inside a PreferencesPage so it gets same margins as other pages
    let example_page = adw::PreferencesPage::new();
    example_page.add(&example_group);

    let right_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    right_box.set_vexpand(true);

    let scroll_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scroll_content.append(&stack);
    scroll_content.append(&example_page);

    let page_scroll = gtk::ScrolledWindow::builder()
        .child(&scroll_content)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    right_box.append(&page_scroll);
    content.append(&right_box);

    root.append(&header);
    root.append(&content);
    editor_dialog.set_child(Some(&root));

    // Cancel
    {
        let gs = gui_state.clone();
        let d = editor_dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::close_editor(&gs);
            d.close();
        });
    }

    // Add
    {
        let state = state.clone();
        let rstore = rule_store.clone();
        let fstore = file_store.clone();
        let gs = gui_state.clone();
        let d = editor_dialog.clone();
        let es = editor_state.clone();
        add_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::add_or_update_rule(&es, &rstore, &fstore, &state, &gs);
            d.close();
        });
    }

    editor_dialog.present(Some(window));
}
