use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use adw::prelude::*;

use crate::state::SharedState;
use super::models::{FileRow, RuleRow};
use super::state_ui::{SharedEditorState, SharedGuiState, SelectMode};
use super::translations_ui::Translations;

fn icon_button(label: &str, icon: &str) -> gtk::Button {
    let btn = gtk::Button::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let img = gtk::Image::from_icon_name(icon);
    img.set_pixel_size(16);
    img.set_margin_start(4);
    content.append(&img);
    let label_widget = gtk::Label::new(Some(label));
    label_widget.set_hexpand(true);
    label_widget.set_xalign(0.5);
    content.append(&label_widget);
    btn.set_child(Some(&content));
    btn
}

pub struct GtkApp {
    pub window: adw::ApplicationWindow,
    pub file_store: gio::ListStore,
    pub rule_store: gio::ListStore,
    pub file_status_label: gtk::Label,
    pub rule_status_label: gtk::Label,
    pub start_button: gtk::Button,
    pub update_button: gtk::Button,
    pub progress_banner: gtk::Revealer,
    pub progress_spinner: gtk::Spinner,
    pub progress_label: gtk::Label,
    pub translations: Rc<RefCell<Translations>>,
    pub editor_state: SharedEditorState,
    pub gui_state: SharedGuiState,
}

pub fn build_gtk_app(
    app: &adw::Application,
    state: SharedState,
    editor_state: SharedEditorState,
    gui_state: SharedGuiState,
    translations: Rc<RefCell<Translations>>,
) -> GtkApp {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Szyszka")
        .default_width(1100)
        .default_height(720)
        .width_request(800)
        .height_request(600)
        .build();

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();

    let start_btn = icon_button(&crate::fls!("upper_start_renaming_button"), "media-playback-start-symbolic");
    start_btn.add_css_class("suggested-action");
    start_btn.set_sensitive(false);
    header.pack_start(&start_btn);

    // Settings hamburger menu button in header
    let menu_model = gio::Menu::new();

    // Preferences entry
    let prefs_section = gio::Menu::new();
    prefs_section.append(Some(&crate::fls!("menu_preferences")), Some("app.open-preferences"));
    menu_model.append_section(None, &prefs_section);

    // Open files section
    let open_section = gio::Menu::new();
    open_section.append(Some(&crate::fls!("menu_open_rules_file")), Some("app.open-rules-file"));
    open_section.append(Some(&crate::fls!("menu_open_custom_texts_file")), Some("app.open-custom-texts-file"));
    open_section.append(Some(&crate::fls!("menu_open_config_dir")), Some("app.open-config-dir"));
    open_section.append(Some(&crate::fls!("menu_open_log_folder")), Some("app.open-log-folder"));
    menu_model.append_section(None, &open_section);

    let menu_popover = gtk::PopoverMenu::from_model(Some(&menu_model));
    let menu_btn = gtk::MenuButton::new();
    menu_btn.set_icon_name("open-menu-symbolic");
    menu_btn.set_tooltip_text(Some("Menu"));
    menu_btn.set_popover(Some(&menu_popover));
    header.pack_end(&menu_btn);

    // Register app actions for menu items
    {
        let w = window.clone();
        let sm = app.style_manager().clone();
        let action = gio::ActionEntry::builder("open-preferences")
            .activate(move |_app: &adw::Application, _, _| {
                show_preferences_dialog(&w, &sm);
            })
            .build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-rules-file")
            .activate(move |_, _, _| {
                if let Some(p) = crate::config::get_rules_config_file() {
                    crate::config::create_rules_file_if_needed();
                    let _ = open::that(p);
                }
            })
            .build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-custom-texts-file")
            .activate(move |_, _, _| {
                if let Some(p) = crate::config::get_custom_text_config_file() {
                    crate::config::create_custom_text_file_if_needed();
                    let _ = open::that(p);
                }
            })
            .build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-config-dir")
            .activate(move |_, _, _| {
                if let Some(p) = crate::config::get_config_path() {
                    let _ = std::fs::create_dir_all(&p);
                    let _ = open::that(p);
                }
            })
            .build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-log-folder")
            .activate(move |_, _, _| {
                if let Some(p) = crate::logger::get_cache_path() {
                    let _ = std::fs::create_dir_all(&p);
                    let _ = open::that(p);
                }
            })
            .build();
        app.add_action_entries([action]);
    }

    toolbar_view.add_top_bar(&header);

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_box.set_margin_top(0);
    main_box.set_margin_bottom(0);
    main_box.set_margin_start(0);
    main_box.set_margin_end(0);

    // Dynamic file status label
    let file_status = gtk::Label::builder().label(&crate::fls!("upper_files_folders_label")).xalign(0.0).build();
    file_status.add_css_class("heading");
    file_status.set_margin_start(12);
    file_status.set_margin_top(8);
    file_status.set_margin_bottom(4);

    // Progress banner (hidden by default)
    let progress_banner = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    let progress_inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    progress_inner.set_margin_top(4);
    progress_inner.set_margin_bottom(4);
    progress_inner.set_margin_start(8);
    progress_inner.set_margin_end(8);
    let progress_spinner = gtk::Spinner::new();
    progress_spinner.set_size_request(16, 16);
    progress_inner.append(&progress_spinner);
    let progress_label = gtk::Label::builder().label(&crate::fls!("dialog_loading")).hexpand(true).xalign(0.0).build();
    progress_inner.append(&progress_label);
    progress_banner.set_child(Some(&progress_inner));
    progress_banner.add_css_class("toolbar");

    let file_store = gio::ListStore::new::<FileRow>();
    let file_selection = gtk::MultiSelection::new(Some(file_store.clone()));
    let (file_column_view, _) = build_file_column_view(&file_selection, &state, &window);
    let file_scroll = gtk::ScrolledWindow::builder().child(&file_column_view).vexpand(true).build();
    file_scroll.add_css_class("card");

    let add_files_btn = icon_button(&crate::fls!("upper_add_files_button"), "document-open-symbolic");
    let add_folders_btn = icon_button(&crate::fls!("upper_add_folders_button"), "folder-open-symbolic");
    let remove_btn = icon_button(&crate::fls!("upper_remove_selection_button"), "edit-delete-symbolic");
    let select_btn = icon_button(&crate::fls!("upper_select_popup_button"), "edit-select-all-symbolic");
    let update_btn = icon_button(&crate::fls!("upper_update_names_button"), "view-refresh-symbolic");
    update_btn.set_sensitive(false);
    let move_up_btn = icon_button(&crate::fls!("dialog_move_up"), "go-up-symbolic");
    let move_down_btn = icon_button(&crate::fls!("dialog_move_down"), "go-down-symbolic");

    let file_action_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    file_action_box.set_margin_start(4);
    file_action_box.set_margin_end(4);
    file_action_box.set_margin_top(4);
    file_action_box.set_margin_bottom(4);
    file_action_box.set_width_request(130);
    for btn in [&add_files_btn, &add_folders_btn, &remove_btn, &select_btn, &update_btn, &move_up_btn, &move_down_btn] {
        btn.set_halign(gtk::Align::Fill);
        file_action_box.append(btn);
    }

    let file_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    file_paned.set_start_child(Some(&file_scroll));
    file_paned.set_end_child(Some(&file_action_box));
    file_paned.set_position(800);

    // Dynamic rule status label
    let rule_status = gtk::Label::builder().label(&crate::fls!("bottom_rule_label_rules")).xalign(0.0).build();
    rule_status.add_css_class("heading");
    rule_status.set_margin_start(12);
    rule_status.set_margin_top(8);
    rule_status.set_margin_bottom(4);

    let rule_store = gio::ListStore::new::<RuleRow>();
    let rule_selection = gtk::MultiSelection::new(Some(rule_store.clone()));
    let rule_column_view = build_rule_column_view(&rule_selection, &state, &editor_state, &rule_store, &gui_state, &window);
    let rule_scroll = gtk::ScrolledWindow::builder().child(&rule_column_view).vexpand(true).build();
    rule_scroll.add_css_class("card");

    let add_rule_btn = icon_button(&crate::fls!("bottom_rule_add_button"), "list-add-symbolic");
    let edit_rule_btn = icon_button(&crate::fls!("bottom_rule_edit_button"), "document-edit-symbolic");
    let remove_rule_btn = icon_button(&crate::fls!("bottom_rule_remove_button"), "list-remove-symbolic");
    let rule_up_btn = icon_button(&crate::fls!("dialog_move_up"), "go-up-symbolic");
    let rule_down_btn = icon_button(&crate::fls!("dialog_move_down"), "go-down-symbolic");
    let load_rules_btn = icon_button(&crate::fls!("bottom_rule_load_rules_button"), "document-open-symbolic");
    let save_rules_btn = icon_button(&crate::fls!("bottom_rule_save_rules_button"), "document-save-symbolic");

    let rule_action_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    rule_action_box.set_margin_start(4);
    rule_action_box.set_margin_end(4);
    rule_action_box.set_margin_top(4);
    rule_action_box.set_margin_bottom(4);
    rule_action_box.set_width_request(130);
    for btn in [&add_rule_btn, &edit_rule_btn, &remove_rule_btn, &rule_up_btn, &rule_down_btn, &load_rules_btn, &save_rules_btn] {
        btn.set_halign(gtk::Align::Fill);
        rule_action_box.append(btn);
    }

    let rule_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    rule_paned.set_start_child(Some(&rule_scroll));
    rule_paned.set_end_child(Some(&rule_action_box));
    rule_paned.set_position(800);

    main_box.append(&progress_banner);
    main_box.append(&file_status);
    main_box.append(&file_paned);
    main_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    main_box.append(&rule_status);
    main_box.append(&rule_paned);

    toolbar_view.set_content(Some(&main_box));
    window.set_content(Some(&toolbar_view));

    let gtk_app = GtkApp {
        window: window.clone(),
        file_store: file_store.clone(),
        rule_store: rule_store.clone(),
        file_status_label: file_status.clone(),
        rule_status_label: rule_status.clone(),
        start_button: start_btn.clone(),
        update_button: update_btn.clone(),
        progress_banner: progress_banner.clone(),
        progress_spinner: progress_spinner.clone(),
        progress_label: progress_label.clone(),
        translations: translations.clone(),
        editor_state: editor_state.clone(),
        gui_state: gui_state.clone(),
    };

    // === Dynamic status labels and progress banner ===
    {
        let file_status_c = file_status.clone();
        let rule_status_c = rule_status.clone();
        let file_store_c = file_store.clone();
        let rule_store_c = rule_store.clone();
        let gs = gui_state.clone();
        let tr = translations.clone();
        let start_btn_c = start_btn.clone();
        let update_btn_c = update_btn.clone();
        let pb = progress_banner.clone();
        let ps = progress_spinner.clone();
        let pl = progress_label.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            let file_count = file_store_c.n_items() as i32;
            let rule_count = rule_store_c.n_items() as i32;
            let outdated = gs.borrow().results_outdated;
            let t = tr.borrow();
            if outdated && file_count > 0 {
                file_status_c.set_label(&format!("{} ({}) - UPDATE REQUIRED", t.upper_files_folders_label, file_count));
            } else if file_count > 0 {
                file_status_c.set_label(&format!("{} ({}) - up to date", t.upper_files_folders_label, file_count));
            } else {
                file_status_c.set_label(&t.upper_files_folders_label);
            }
            start_btn_c.set_sensitive(file_count > 0 && rule_count > 0);
            update_btn_c.set_sensitive(file_count > 0);
            if rule_count > 0 {
                rule_status_c.set_label(&format!("{} ({})", t.bottom_rule_label_rules, rule_count));
            } else {
                rule_status_c.set_label(&t.bottom_rule_label_rules);
            }
            // Progress banner: show when async_active or when there's a message
            let active = gs.borrow().message_dialog_title.len() > 0;
            pb.set_reveal_child(active);
            if active {
                ps.set_spinning(active);
                pl.set_label(&gs.borrow().message_dialog_title);
            }
            glib::ControlFlow::Continue
        });
    }

    // === Select button (popup) ===
    {
        let window = window.clone();
        let file_store = file_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        select_btn.connect_clicked(move |_| {
            show_select_popup(&window, &file_store, &state, &gs);
        });
    }

    // === Start renaming ===
    {
        let window = window.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        start_btn.connect_clicked(move |_| {
            crate::connect::renaming::start_renaming_request(&window, &state, &gs);
        });
    }

    // === Add files ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        add_files_btn.connect_clicked(move |_| {
            crate::connect::files::pick_files_and_add(&state, &file_store, &gs);
        });
    }

    // === Add folders (with proper dialog) ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        let window = window.clone();
        add_folders_btn.connect_clicked(move |_| {
            if crate::connect::files::pick_folders_into_state(&state, &gs) {
                show_add_folders_dialog(&window, &state, &file_store, &gs);
            }
        });
    }

    // === Remove selected files ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        remove_btn.connect_clicked(move |_| {
            crate::connect::files::remove_selected(&state, &file_store, &gs);
        });
    }

    // === Update names ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        update_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::refresh_future_names(&state);
            state.borrow_mut().rules.updated = true;
            crate::connect::sync::sync_files(&file_store, &state);
            crate::connect::sync::sync_outdated(&gs, &state);
        });
    }

    // === Move file up/down ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        move_up_btn.connect_clicked(move |_| {
            crate::connect::files::move_selected_up(&state, &file_store);
        });
    }
    {
        let state = state.clone();
        let file_store = file_store.clone();
        move_down_btn.connect_clicked(move |_| {
            crate::connect::files::move_selected_down(&state, &file_store);
        });
    }

    // === Rule operations ===
    {
        let window = window.clone();
        let es = editor_state.clone();
        let state = state.clone();
        let rule_store = rule_store.clone();
        let gs = gui_state.clone();
        add_rule_btn.connect_clicked(move |_| {
            super::rule_editor::show_rule_editor(&window, &es, &state, &rule_store, &gs, None);
        });
    }
    {
        let window = window.clone();
        let es = editor_state.clone();
        let state = state.clone();
        let rule_store = rule_store.clone();
        let gs = gui_state.clone();
        edit_rule_btn.connect_clicked(move |_| {
            let idx = state.borrow().rule_selected.iter().position(|x| *x).map(|i| i as i32).unwrap_or(-1);
            super::rule_editor::show_rule_editor(&window, &es, &state, &rule_store, &gs, Some(idx));
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        remove_rule_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::remove_rule(&rule_store, &state, &gs, -1);
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        rule_up_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::move_rule_up(&rule_store, &state, &gs);
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        rule_down_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::move_rule_down(&rule_store, &state, &gs);
        });
    }

    // === Save rules ===
    {
        let state = state.clone();
        let gs = gui_state.clone();
        let window = window.clone();
        save_rules_btn.connect_clicked(move |_| {
            let dialog = adw::AlertDialog::builder()
                .heading(&crate::fls!("dialog_save_rule_set_title"))
                .body(&crate::fls!("dialog_save_rule_set_body"))
                .build();
            dialog.add_response("cancel", &crate::fls!("dialog_button_cancel"));
            dialog.add_response("save", &crate::fls!("dialog_button_ok"));
            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
            let entry = gtk::Entry::builder().placeholder_text(&crate::fls!("dialog_save_rule_set_name")).build();
            dialog.set_extra_child(Some(&entry));
            let st = state.clone();
            let gs2 = gs.clone();
            dialog.connect_response(Some("save"), move |_, _| {
                let name = entry.text().to_string();
                crate::connect::rules_ops::save_rule_set(&st, &name);
                crate::connect::rules_ops::refresh_rule_sets(&gs2);
            });
            dialog.present(Some(&window));
        });
    }

    // === Load rules ===
    {
        let state = state.clone();
        let rule_store = rule_store.clone();
        let gs = gui_state.clone();
        let window = window.clone();
        load_rules_btn.connect_clicked(move |_| {
            let all = crate::config::load_rules();
            if all.is_empty() {
                return;
            }
            let dialog = adw::Dialog::builder()
                .title(&crate::fls!("dialog_saved_rule_sets"))
                .content_width(400)
                .content_height(400)
                .build();

            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
            vbox.set_margin_top(12);
            vbox.set_margin_bottom(12);
            vbox.set_margin_start(12);
            vbox.set_margin_end(12);

            let list_box = gtk::ListBox::new();
            list_box.add_css_class("boxed-list");
            for (i, entry) in all.iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(&entry.name)
                    .activatable(false)
                    .build();
                let load_btn = gtk::Button::with_label(&crate::fls!("rule_editor_load"));
                load_btn.set_size_request(32, 32);
                load_btn.set_vexpand(false);
                load_btn.set_valign(gtk::Align::Center);
                load_btn.add_css_class("flat");
                load_btn.add_css_class("suggested-action");
                let st = state.clone();
                let store2 = rule_store.clone();
                let gs2 = gs.clone();
                let d = dialog.clone();
                let idx = i as i32;
                load_btn.connect_clicked(move |_| {
                    crate::connect::rules_ops::load_rule_set(&store2, &st, &gs2, idx);
                    d.close();
                });
                row.add_suffix(&load_btn);

                let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                del_btn.set_size_request(32, 32);
                del_btn.set_vexpand(false);
                del_btn.set_valign(gtk::Align::Center);
                del_btn.add_css_class("flat");
                del_btn.add_css_class("destructive-action");
                let d2 = dialog.clone();
                let idx = i as i32;
                del_btn.connect_clicked(move |_| {
                    crate::connect::rules_ops::delete_rule_set(idx);
                    d2.close();
                });
                row.add_suffix(&del_btn);
                list_box.append(&row);
            }
            vbox.append(&list_box);

            let close_btn = gtk::Button::with_label(&crate::fls!("dialog_button_cancel"));
            close_btn.set_halign(gtk::Align::End);
            {
                let d = dialog.clone();
                close_btn.connect_clicked(move |_| { d.close(); });
            }
            vbox.append(&close_btn);

            dialog.set_child(Some(&vbox));
            dialog.present(Some(&window));
        });
    }

    gtk_app
}

fn show_preferences_dialog(window: &adw::ApplicationWindow, style_manager: &adw::StyleManager) {
    let dialog = adw::PreferencesDialog::new();

    // Appearance page
    let appearance_page = adw::PreferencesPage::new();
    let appearance_group = adw::PreferencesGroup::builder()
        .title(&crate::fls!("menu_appearance"))
        .build();

    // Theme selection
    let theme_row = adw::ComboRow::builder()
        .title(&crate::fls!("settings_theme"))
        .model(&gtk::StringList::new(&[
            &crate::fls!("settings_theme_system"),
            &crate::fls!("settings_theme_light"),
            &crate::fls!("settings_theme_dark"),
        ]))
        .selected(match style_manager.color_scheme() {
            libadwaita::ColorScheme::ForceDark => 2,
            libadwaita::ColorScheme::ForceLight => 1,
            _ => 0,
        })
        .build();
    {
        let sm = style_manager.clone();
        theme_row.connect_selected_notify(move |row| {
            let scheme = match row.selected() {
                2 => libadwaita::ColorScheme::ForceDark,
                1 => libadwaita::ColorScheme::ForceLight,
                _ => libadwaita::ColorScheme::Default,
            };
            sm.set_color_scheme(scheme);
            crate::config::save_dark_theme(scheme == libadwaita::ColorScheme::ForceDark);
        });
    }
    appearance_group.add(&theme_row);
    appearance_page.add(&appearance_group);
    dialog.add(&appearance_page);

    // Language page
    let language_page = adw::PreferencesPage::new();
    let language_group = adw::PreferencesGroup::builder()
        .title(&crate::fls!("settings_language_label"))
        .build();

    let saved_lang = crate::config::load_saved_language();
    let lang_combo = adw::ComboRow::builder()
        .title(&crate::fls!("settings_language_label"))
        .model(&gtk::StringList::new(
            &crate::language::LANGUAGES_ALL.iter().map(|l| l.combo_box_text).collect::<Vec<_>>()
        ))
        .selected(
            crate::language::LANGUAGES_ALL.iter()
                .position(|l| l.combo_box_text == saved_lang)
                .unwrap_or(0) as u32
        )
        .build();
    {
        let w = window.clone();
        lang_combo.connect_selected_notify(move |row| {
            let lang = &crate::language::LANGUAGES_ALL[row.selected() as usize];
            let current = crate::config::load_saved_language();
            if lang.combo_box_text != current {
                crate::config::save_language(lang.combo_box_text);
                crate::language::apply_language(lang.combo_box_text);

                let confirm = adw::AlertDialog::builder()
                    .heading(&crate::fls!("dialog_language_title"))
                    .body(&crate::fls!("dialog_language_restart_confirm"))
                    .build();
                confirm.add_response("cancel", &crate::fls!("dialog_button_cancel"));
                confirm.add_response("restart", &crate::fls!("dialog_language_restart"));
                confirm.set_response_appearance("restart", adw::ResponseAppearance::Suggested);
                let w2 = w.clone();
                confirm.connect_response(Some("restart"), move |_, _| {
                    w2.close();
                    let app = w2.application().unwrap();
                    app.activate();
                });
                confirm.present(Some(&w));
            }
        });
    }
    language_group.add(&lang_combo);
    language_page.add(&language_group);
    dialog.add(&language_page);

    dialog.present(Some(window));
}

fn show_select_popup(window: &adw::ApplicationWindow, file_store: &gio::ListStore, state: &SharedState, gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder()
        .heading(&crate::fls!("dialog_select"))
        .body(&crate::fls!("dialog_select_body"))
        .build();

    dialog.add_response("all", &crate::fls!("button_select_all"));
    dialog.add_response("none", &crate::fls!("button_unselect_all"));
    dialog.add_response("reverse", &crate::fls!("button_select_reverse"));
    dialog.add_response("custom", &crate::fls!("button_select_custom"));
    dialog.add_response("changed", &crate::fls!("button_select_changed"));
    dialog.add_response("unchanged", &crate::fls!("button_unselect_changed"));

    let store = file_store.clone();
    let st = state.clone();
    let w = window.clone();
    let gs = gui_state.clone();
    dialog.connect_response(None, move |_, response| {
        match response {
            "all" => crate::connect::select::apply_select(&store, &st, SelectMode::SelectAll),
            "none" => crate::connect::select::apply_select(&store, &st, SelectMode::UnselectAll),
            "reverse" => crate::connect::select::apply_select(&store, &st, SelectMode::Reverse),
            "changed" => crate::connect::select::apply_select(&store, &st, SelectMode::SelectChanged),
            "unchanged" => crate::connect::select::apply_select(&store, &st, SelectMode::UnselectChanged),
            "custom" => {
                show_select_custom_dialog(&w, &store, &st, &gs);
            }
            _ => {}
        }
    });
    dialog.present(Some(window));
}

pub fn show_select_custom_dialog(window: &adw::ApplicationWindow, file_store: &gio::ListStore, state: &SharedState, _gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder()
        .heading(&crate::fls!("dialog_select_custom_title"))
        .body(&crate::fls!("dialog_select_custom_body"))
        .build();

    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);
    content_box.set_margin_start(8);
    content_box.set_margin_end(8);

    let pattern_entry = gtk::Entry::builder().placeholder_text(&crate::fls!("dialog_select_custom_pattern")).hexpand(true).build();
    content_box.append(&pattern_entry);

    let include_dirs_check = gtk::CheckButton::with_label(&crate::fls!("dialog_select_custom_include_dirs"));
    include_dirs_check.set_active(true);
    content_box.append(&include_dirs_check);

    let mode_label = gtk::Label::builder().label("Match against:").xalign(0.0).build();
    content_box.append(&mode_label);
    let mode_combo = gtk::DropDown::from_strings(&[
        "Path",
        "Current Name",
        "Future Name",
        "Path + Current Name",
        "Path + Future Name",
        "Directory / File",
    ]);
    content_box.append(&mode_combo);

    let hint_label = gtk::Label::builder()
        .label("When Directory/File mode is active, pattern is ignored.")
        .wrap(true)
        .xalign(0.0)
        .build();
    hint_label.add_css_class("dim-label");
    content_box.append(&hint_label);

    dialog.set_extra_child(Some(&content_box));

    dialog.add_response("cancel", &crate::fls!("dialog_button_cancel"));
    dialog.add_response("select", &crate::fls!("button_select_custom"));
    dialog.add_response("unselect", &crate::fls!("button_unselect_all"));
    dialog.set_response_appearance("select", adw::ResponseAppearance::Suggested);

    let store = file_store.clone();
    let st = state.clone();
    let pe = pattern_entry.clone();
    let idc = include_dirs_check.clone();
    let mc = mode_combo.clone();
    dialog.connect_response(Some("select"), move |_, _| {
        let pattern = pe.text().to_string();
        let include_dirs = idc.is_active();
        let mode_index = mc.selected() as i32;
        crate::connect::select::apply_select_custom(&store, &st, &pattern, include_dirs, mode_index, true);
    });
    let store = file_store.clone();
    let st = state.clone();
    let pe = pattern_entry.clone();
    let idc = include_dirs_check.clone();
    let mc = mode_combo.clone();
    dialog.connect_response(Some("unselect"), move |_, _| {
        let pattern = pe.text().to_string();
        let include_dirs = idc.is_active();
        let mode_index = mc.selected() as i32;
        crate::connect::select::apply_select_custom(&store, &st, &pattern, include_dirs, mode_index, false);
    });

    dialog.present(Some(window));
}

fn show_add_folders_dialog(window: &adw::ApplicationWindow, state: &SharedState, file_store: &gio::ListStore, gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder()
        .heading(&crate::fls!("dialog_add_folders_title"))
        .body(&crate::fls!("dialog_add_folders_body"))
        .build();

    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);
    content_box.set_margin_start(8);
    content_box.set_margin_end(8);

    // Show picked paths
    let paths_label = {
        let gs = gui_state.borrow();
        let paths = &gs.add_folder_picked_paths;
        let text = if paths.is_empty() {
            "No folders selected".to_string()
        } else {
            paths.join("\n")
        };
        gtk::Label::builder().label(&text).xalign(0.0).wrap(true).build()
    };
    content_box.append(&paths_label);

    let scan_check = gtk::CheckButton::with_label(&crate::fls!("dialog_scan_inside"));
    let ignore_check = gtk::CheckButton::with_label(&crate::fls!("dialog_ignore_folders"));
    content_box.append(&scan_check);
    content_box.append(&ignore_check);

    dialog.set_extra_child(Some(&content_box));
    dialog.add_response("cancel", &crate::fls!("dialog_button_cancel"));
    dialog.add_response("ok", &crate::fls!("dialog_button_ok"));
    dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);

    let st = state.clone();
    let store = file_store.clone();
    let gs = gui_state.clone();
    dialog.connect_response(Some("ok"), move |_, _| {
        let scan_inside = scan_check.is_active();
        let ignore_folders = ignore_check.is_active();
        crate::connect::files::confirm_add_folders(&st, &store, &gs, scan_inside, ignore_folders);
    });
    dialog.present(Some(window));
}

fn build_file_column_view(
    selection: &gtk::MultiSelection,
    state: &SharedState,
    _window: &adw::ApplicationWindow,
) -> (gtk::ColumnView, gio::ListStore) {
    let column_view = gtk::ColumnView::new(Some(selection.clone()));
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_single_click_activate(false);

    let make_factory = |col_idx: usize| {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_margin_start(6);
            label.set_margin_end(6);
            li.set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = li.item().and_downcast::<FileRow>().unwrap();
            let label = li.child().and_downcast::<gtk::Label>().unwrap();
            let text = match col_idx {
                0 => if row.is_dir() { "Dir" } else { "File" }.to_string(),
                1 => row.current_name(),
                2 => row.future_name(),
                3 => row.path(),
                _ => String::new(),
            };
            label.set_label(&text);
        });
        factory
    };

    let type_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_type")), Some(make_factory(0)));
    type_col.set_fixed_width(50);
    let type_sorter = gtk::CustomSorter::new(|a, b| {
        let a = a.downcast_ref::<FileRow>().unwrap();
        let b = b.downcast_ref::<FileRow>().unwrap();
        a.is_dir().cmp(&b.is_dir()).into()
    });
    type_col.set_sorter(Some(&type_sorter));
    column_view.append_column(&type_col);

    let current_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_current_name")), Some(make_factory(1)));
    current_col.set_expand(true);
    let current_sorter = gtk::CustomSorter::new(|a, b| {
        let a = a.downcast_ref::<FileRow>().unwrap();
        let b = b.downcast_ref::<FileRow>().unwrap();
        natord::compare(&a.current_name(), &b.current_name()).into()
    });
    current_col.set_sorter(Some(&current_sorter));
    column_view.append_column(&current_col);

    let future_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_future_name")), Some(make_factory(2)));
    future_col.set_expand(true);
    let future_sorter = gtk::CustomSorter::new(|a, b| {
        let a = a.downcast_ref::<FileRow>().unwrap();
        let b = b.downcast_ref::<FileRow>().unwrap();
        natord::compare(&a.future_name(), &b.future_name()).into()
    });
    future_col.set_sorter(Some(&future_sorter));
    column_view.append_column(&future_col);

    let path_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_path")), Some(make_factory(3)));
    path_col.set_expand(true);
    let path_sorter = gtk::CustomSorter::new(|a, b| {
        let a = a.downcast_ref::<FileRow>().unwrap();
        let b = b.downcast_ref::<FileRow>().unwrap();
        natord::compare(&a.path(), &b.path()).into()
    });
    path_col.set_sorter(Some(&path_sorter));
    column_view.append_column(&path_col);

    // Column header sorting - use sort-by-column when sorter changes
    {
        let st = state.clone();
        let store = selection.model().and_downcast::<gio::ListStore>().unwrap();
        let type_col_ref = type_col.clone();
        let current_col_ref = current_col.clone();
        let future_col_ref = future_col.clone();
        let path_col_ref = path_col.clone();
        let cv_ref = column_view.clone();

        // Track sort state to detect changes
        let last_sort: std::rc::Rc<std::cell::Cell<i32>> = std::rc::Rc::new(std::cell::Cell::new(-1));
        let last_desc: std::rc::Rc<std::cell::Cell<bool>> = std::rc::Rc::new(std::cell::Cell::new(false));

        column_view.connect_sorter_notify(move |cv| {
            let sorter = cv.sorter();
            // Get sort direction from ColumnView's sort columns
            let sort_col = type_col_ref.sorter().as_ref().map(|s| s as *const _);
            // Simply sort the data by cycling through columns on each click
            let current = last_sort.get();
            let (key, desc) = match current {
                0 => {
                    if last_desc.get() {
                        (crate::connect::files::SortKey::None, false)
                    } else {
                        (crate::connect::files::SortKey::Type, true)
                    }
                }
                1 => {
                    if last_desc.get() {
                        (crate::connect::files::SortKey::Type, false)
                    } else {
                        (crate::connect::files::SortKey::Current, true)
                    }
                }
                2 => {
                    if last_desc.get() {
                        (crate::connect::files::SortKey::Current, false)
                    } else {
                        (crate::connect::files::SortKey::Future, true)
                    }
                }
                3 => {
                    if last_desc.get() {
                        (crate::connect::files::SortKey::Future, false)
                    } else {
                        (crate::connect::files::SortKey::Path, true)
                    }
                }
                _ => {
                    (crate::connect::files::SortKey::Type, false)
                }
            };

            let new_idx = match key {
                crate::connect::files::SortKey::None => -1,
                crate::connect::files::SortKey::Type => 0,
                crate::connect::files::SortKey::Current => 1,
                crate::connect::files::SortKey::Future => 2,
                crate::connect::files::SortKey::Path => 3,
            };
            last_sort.set(new_idx);
            last_desc.set(desc);

            if key != crate::connect::files::SortKey::None {
                crate::connect::files::sort_files_by(&st, &store, key, desc);
            }
            let _ = sorter;
        });
    }

    // Double-click to open file
    {
        let st = state.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(1);
        gesture.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                let s = st.borrow();
                if let Some(idx) = s.file_selected.iter().position(|x| *x) {
                    if let Some(item) = s.files.get(idx) {
                        let _ = open::that(&item.full_name);
                    }
                }
            }
        });
        column_view.add_controller(gesture);
    }

    // Right-click to open containing folder
    {
        let st = state.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(move |_, _, _, _| {
            let s = st.borrow();
            if let Some(idx) = s.file_selected.iter().position(|x| *x) {
                if let Some(item) = s.files.get(idx) {
                    let _ = open::that(&item.path);
                }
            }
        });
        column_view.add_controller(gesture);
    }

    (column_view, selection.model().and_downcast::<gio::ListStore>().unwrap())
}

fn build_rule_column_view(
    selection: &gtk::MultiSelection,
    state: &SharedState,
    editor_state: &SharedEditorState,
    rule_store: &gio::ListStore,
    gui_state: &SharedGuiState,
    window: &adw::ApplicationWindow,
) -> gtk::ColumnView {
    let column_view = gtk::ColumnView::new(Some(selection.clone()));
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_single_click_activate(false);

    let make_factory = |col_idx: usize| {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_margin_start(6);
            label.set_margin_end(6);
            li.set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = li.item().and_downcast::<RuleRow>().unwrap();
            let label = li.child().and_downcast::<gtk::Label>().unwrap();
            let text = match col_idx {
                0 => row.rule_type_text(),
                1 => row.usage_text(),
                2 => row.description(),
                _ => String::new(),
            };
            label.set_label(&text);
        });
        factory
    };

    let tool_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_tool_type")), Some(make_factory(0)));
    tool_col.set_expand(true);
    column_view.append_column(&tool_col);

    let usage_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_usage_name")), Some(make_factory(1)));
    usage_col.set_expand(true);
    column_view.append_column(&usage_col);

    let desc_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_description")), Some(make_factory(2)));
    desc_col.set_expand(true);
    column_view.append_column(&desc_col);

    // Double-click to edit rule
    {
        let st = state.clone();
        let es = editor_state.clone();
        let rs = rule_store.clone();
        let gs = gui_state.clone();
        let w = window.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(1);
        gesture.connect_released(move |_, n_press, _, _| {
            if n_press >= 2 {
                let idx = st.borrow().rule_selected.iter().position(|x| *x).map(|i| i as i32).unwrap_or(0);
                super::rule_editor::show_rule_editor(&w, &es, &st, &rs, &gs, Some(idx));
            }
        });
        column_view.add_controller(gesture);
    }

    column_view
}
