use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk::prelude::*;
use adw::prelude::*;

use crate::state::SharedState;
use super::models::{FileRow, RuleRow};
use super::state_ui::{SharedEditorState, SharedGuiState, SelectMode};
use super::translations_ui::Translations;

fn icon_button(label: &str, icon: &str) -> gtk::Button {
    let btn = gtk::Button::new();
    btn.set_hexpand(true);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_halign(gtk::Align::Center);
    let img = gtk::Image::from_icon_name(icon);
    img.set_pixel_size(16);
    content.append(&img);
    let label_widget = gtk::Label::new(Some(label));
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
    pub progress_banner: adw::Banner,
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

    // Custom CSS for green future name highlighting and selection styling
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data(
        ".future-name-changed { color: #2e8b57; }
         columnview > listview > row:selected {
             background: @accent_bg_color;
         }
         columnview > listview > row:selected label {
             color: @accent_fg_color;
         }
         .drop-hover {
             outline: 2px solid @accent_bg_color;
             outline-offset: -2px;
         }"
    );
    // Will be added to display after window is realized

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();

    // HeaderBar: Start Renaming (left), Add Files, Add Folders (right), Menu (end)
    let start_btn = icon_button(&crate::fls!("upper_start_renaming_button"), "media-playback-start-symbolic");
    start_btn.add_css_class("suggested-action");
    start_btn.set_sensitive(false);
    header.pack_start(&start_btn);

    let add_files_btn = icon_button(&crate::fls!("upper_add_files_button"), "document-open-symbolic");
    let add_folders_btn = icon_button(&crate::fls!("upper_add_folders_button"), "folder-open-symbolic");

    // Hamburger menu
    let menu_model = gio::Menu::new();
    let prefs_section = gio::Menu::new();
    prefs_section.append(Some(&crate::fls!("menu_preferences")), Some("app.open-preferences"));
    menu_model.append_section(None, &prefs_section);
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

    // Register app actions
    {
        let w = window.clone();
        let sm = app.style_manager().clone();
        let action = gio::ActionEntry::builder("open-preferences")
            .activate(move |_, _, _| show_preferences_dialog(&w, &sm))
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
            }).build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-custom-texts-file")
            .activate(move |_, _, _| {
                if let Some(p) = crate::config::get_custom_text_config_file() {
                    crate::config::create_custom_text_file_if_needed();
                    let _ = open::that(p);
                }
            }).build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-config-dir")
            .activate(move |_, _, _| {
                if let Some(p) = crate::config::get_config_path() {
                    let _ = std::fs::create_dir_all(&p);
                    let _ = open::that(p);
                }
            }).build();
        app.add_action_entries([action]);
    }
    {
        let action = gio::ActionEntry::builder("open-log-folder")
            .activate(move |_, _, _| {
                if let Some(p) = crate::logger::get_cache_path() {
                    let _ = std::fs::create_dir_all(&p);
                    let _ = open::that(p);
                }
            }).build();
        app.add_action_entries([action]);
    }

    toolbar_view.add_top_bar(&header);

    // Main content
    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Progress banner
    let progress_banner = adw::Banner::new(&crate::fls!("dialog_loading"));
    progress_banner.set_revealed(false);
    main_box.append(&progress_banner);

    // File status label
    let file_status = gtk::Label::builder().label(&crate::fls!("upper_files_folders_label")).xalign(0.0).build();
    file_status.add_css_class("heading");
    file_status.set_margin_start(12);
    file_status.set_margin_top(8);
    file_status.set_margin_bottom(4);
    main_box.append(&file_status);

    // File list with empty state
    let (file_column_view, file_store, file_selection) = build_file_column_view(&state, &window);
    let file_scroll = gtk::ScrolledWindow::builder().child(&file_column_view).vexpand(true).build();
    file_scroll.add_css_class("card");

    let file_empty_page = adw::StatusPage::builder()
        .icon_name("folder-documents-symbolic")
        .title(&crate::fls!("empty_state_files_title"))
        .valign(gtk::Align::Center)
        .hexpand(true)
        .vexpand(true)
        .build();

    let file_stack = gtk::Stack::new();
    file_stack.set_vexpand(true);
    file_stack.set_overflow(gtk::Overflow::Hidden);
    file_stack.add_named(&file_scroll, Some("list"));
    file_stack.add_named(&file_empty_page, Some("empty"));
    file_stack.set_visible_child_name("empty");
    main_box.append(&file_stack);

    // === Drag and drop to add files/folders ===
    {
        use std::path::PathBuf;
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        // Accept GdkFileList (the format GNOME/Nautilus and other GTK file
        // managers provide when dragging files) plus a single GFile.
        let drop_target = gtk::DropTarget::new(gtk::gdk::FileList::static_type(), gtk::gdk::DragAction::COPY);
        drop_target.set_types(&[gtk::gdk::FileList::static_type(), gio::File::static_type()]);

        let hover_stack = file_stack.clone();
        drop_target.connect_enter(move |_, _, _| {
            hover_stack.add_css_class("drop-hover");
            gtk::gdk::DragAction::COPY
        });
        let hover_stack = file_stack.clone();
        drop_target.connect_leave(move |_| {
            hover_stack.remove_css_class("drop-hover");
        });

        let drop_stack = file_stack.clone();
        let drop_window = window.clone();
        drop_target.connect_drop(move |_, value, _, _| {
            let mut files: Vec<PathBuf> = Vec::new();
            let mut folders: Vec<PathBuf> = Vec::new();

            // GdkFileList: preferred format from GTK/Nautilus file managers
            if let Ok(list) = value.get::<gtk::gdk::FileList>() {
                for f in list.files() {
                    if let Some(p) = f.path() {
                        if p.is_dir() { folders.push(p); } else { files.push(p); }
                    }
                }
            }
            // Single GFile
            else if let Ok(file) = value.get::<gio::File>() {
                if let Some(p) = file.path() {
                    if p.is_dir() { folders.push(p); } else { files.push(p); }
                }
            }
            // text/uri-list: fallback used by some (non-GTK) drag sources
            else if let Ok(uris) = value.get::<String>() {
                for uri in uris.split(|c| c == '\r' || c == '\n').filter(|s| !s.is_empty()) {
                    let file = gio::File::for_uri(uri);
                    if let Some(p) = file.path() {
                        if p.is_dir() { folders.push(p); } else { files.push(p); }
                    }
                }
            }

            if files.is_empty() && folders.is_empty() {
                return false;
            }

            // Files are added immediately, just like the "Add files" button.
            if !files.is_empty() {
                let items = crate::files::sort_files(files);
                crate::connect::files::start_async_scan(&items, &state, &file_store, &gs, "Adding files…");
            }

            // Folders prompt the same "configure scan options" dialog as the
            // "Add folders" button, instead of being scanned automatically.
            if !folders.is_empty() {
                let display: Vec<String> = folders.iter().map(|p| p.display().to_string()).collect();
                gs.borrow_mut().add_folder_picked_paths = display;
                state.borrow_mut().pending_folders = folders;
                show_add_folders_dialog(&drop_window, &state, &file_store, &gs);
            }

            drop_stack.remove_css_class("drop-hover");
            true
        });
        // Attach to the top-level window so the controller reliably receives
        // drag events anywhere in the app.
        window.add_controller(drop_target);
    }

    // File action bar (bottom of file list)
    let file_action_bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    file_action_bar.set_margin_top(4);
    file_action_bar.set_margin_bottom(4);
    file_action_bar.set_margin_start(4);
    file_action_bar.set_margin_end(4);
    let remove_btn = icon_button(&crate::fls!("upper_remove_selection_button"), "edit-delete-symbolic");
    let select_btn = icon_button(&crate::fls!("upper_select_popup_button"), "edit-select-all-symbolic");
    let update_btn = icon_button(&crate::fls!("upper_update_names_button"), "view-refresh-symbolic");
    update_btn.set_sensitive(false);
    let file_main_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    file_main_box.set_homogeneous(true);
    file_main_box.set_hexpand(true);
    file_main_box.append(&add_files_btn);
    file_main_box.append(&add_folders_btn);
    file_main_box.append(&remove_btn);
    file_main_box.append(&select_btn);
    file_main_box.append(&update_btn);
    file_action_bar.append(&file_main_box);
    let move_up_btn = gtk::Button::from_icon_name("go-up-symbolic");
    move_up_btn.set_tooltip_text(Some("Move Up"));
    move_up_btn.set_hexpand(false);
    let move_down_btn = gtk::Button::from_icon_name("go-down-symbolic");
    move_down_btn.set_tooltip_text(Some("Move Down"));
    move_down_btn.set_hexpand(false);
    let move_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    move_box.append(&move_up_btn);
    move_box.append(&move_down_btn);
    file_action_bar.append(&move_box);
    main_box.append(&file_action_bar);

    // Rule status label
    let rule_status = gtk::Label::builder().label(&crate::fls!("bottom_rule_label_rules")).xalign(0.0).build();
    rule_status.add_css_class("heading");
    rule_status.set_margin_start(12);
    rule_status.set_margin_top(8);
    rule_status.set_margin_bottom(4);
    main_box.append(&rule_status);

    // Rule list with empty state
    let rule_store = gio::ListStore::new::<RuleRow>();
    let rule_selection = gtk::MultiSelection::new(Some(rule_store.clone()));
    state.borrow_mut().rule_selection = Some(rule_selection.clone());
    let rule_column_view = build_rule_column_view(&rule_selection, &state, &editor_state, &rule_store, &file_store, &gui_state, &window);
    let rule_scroll = gtk::ScrolledWindow::builder().child(&rule_column_view).vexpand(true).build();
    rule_scroll.add_css_class("card");

    let rule_empty_page = adw::StatusPage::builder()
        .icon_name("text-x-generic-symbolic")
        .title(&crate::fls!("empty_state_rules_title"))
        .valign(gtk::Align::Center)
        .hexpand(true)
        .vexpand(true)
        .build();

    let rule_stack = gtk::Stack::new();
    rule_stack.set_vexpand(true);
    rule_stack.set_overflow(gtk::Overflow::Hidden);
    rule_stack.add_named(&rule_scroll, Some("list"));
    rule_stack.add_named(&rule_empty_page, Some("empty"));
    rule_stack.set_visible_child_name("empty");
    main_box.append(&rule_stack);

    // Rule action bar (bottom of rule list)
    let rule_action_bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    rule_action_bar.set_margin_top(4);
    rule_action_bar.set_margin_bottom(4);
    rule_action_bar.set_margin_start(4);
    rule_action_bar.set_margin_end(4);
    let add_rule_btn = icon_button(&crate::fls!("bottom_rule_add_button"), "list-add-symbolic");
    let edit_rule_btn = icon_button(&crate::fls!("bottom_rule_edit_button"), "document-edit-symbolic");
    let remove_rule_btn = icon_button(&crate::fls!("bottom_rule_remove_button"), "list-remove-symbolic");
    let rule_up_btn = gtk::Button::from_icon_name("go-up-symbolic");
    rule_up_btn.set_tooltip_text(Some("Move Up"));
    rule_up_btn.set_hexpand(false);
    let rule_down_btn = gtk::Button::from_icon_name("go-down-symbolic");
    rule_down_btn.set_tooltip_text(Some("Move Down"));
    rule_down_btn.set_hexpand(false);
    let load_rules_btn = icon_button(&crate::fls!("bottom_rule_load_rules_button"), "document-open-symbolic");
    let save_rules_btn = icon_button(&crate::fls!("bottom_rule_save_rules_button"), "document-save-symbolic");
    let rule_main_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    rule_main_box.set_homogeneous(true);
    rule_main_box.set_hexpand(true);
    rule_main_box.append(&add_rule_btn);
    rule_main_box.append(&edit_rule_btn);
    rule_main_box.append(&remove_rule_btn);
    rule_main_box.append(&load_rules_btn);
    rule_main_box.append(&save_rules_btn);
    rule_action_bar.append(&rule_main_box);
    let rule_move_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    rule_move_box.append(&rule_up_btn);
    rule_move_box.append(&rule_down_btn);
    rule_action_bar.append(&rule_move_box);
    main_box.append(&rule_action_bar);

    toolbar_view.set_content(Some(&main_box));
    window.set_content(Some(&toolbar_view));

    // Add CSS provider to display when realized
    {
        let cp = css_provider.clone();
        window.connect_realize(move |w| {
            let display = gtk::prelude::WidgetExt::display(w);
            gtk::style_context_add_provider_for_display(
                &display,
                &cp,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });
    }

    let gtk_app = GtkApp {
        window: window.clone(),
        file_store: file_store.clone(),
        rule_store: rule_store.clone(),
        file_status_label: file_status,
        rule_status_label: rule_status,
        start_button: start_btn.clone(),
        update_button: update_btn.clone(),
        progress_banner: progress_banner.clone(),
        translations: translations.clone(),
        editor_state: editor_state.clone(),
        gui_state: gui_state.clone(),
    };

    // === Dynamic status labels ===
    {
        let file_status_c = gtk_app.file_status_label.clone();
        let rule_status_c = gtk_app.rule_status_label.clone();
        let file_store_c = file_store.clone();
        let rule_store_c = rule_store.clone();
        let gs = gui_state.clone();
        let tr = translations.clone();
        let start_btn_c = start_btn.clone();
        let update_btn_c = update_btn.clone();
        let pb = progress_banner.clone();
        let file_stack_c = file_stack.clone();
        let rule_stack_c = rule_stack.clone();
        let w = window.clone();
        let progress_dialog_cell = std::rc::Rc::new(std::cell::RefCell::new(None::<adw::Dialog>));
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            let file_count = file_store_c.n_items() as i32;
            let rule_count = rule_store_c.n_items() as i32;
            let outdated = gs.borrow().results_outdated;
            let t = tr.borrow();
            if outdated && file_count > 0 {
                file_status_c.set_label(&format!("{} ({}) - {}", t.upper_files_folders_label, file_count, crate::fls!("status_update_required")));
            } else if file_count > 0 {
                file_status_c.set_label(&format!("{} ({}) - {}", t.upper_files_folders_label, file_count, crate::fls!("status_up_to_date")));
            } else {
                file_status_c.set_label(&t.upper_files_folders_label);
            }
            start_btn_c.set_sensitive(file_count > 0 && rule_count > 0);
            update_btn_c.set_sensitive(file_count > 0);
            file_stack_c.set_visible_child_name(if file_count > 0 { "list" } else { "empty" });
            if rule_count > 0 {
                rule_status_c.set_label(&format!("{} ({})", t.bottom_rule_label_rules, rule_count));
            } else {
                rule_status_c.set_label(&t.bottom_rule_label_rules);
            }
            rule_stack_c.set_visible_child_name(if rule_count > 0 { "list" } else { "empty" });

            // Progress dialog (blocking, determinate)
            let progress_active = gs.borrow().progress_active;
            if progress_active {
                let title = gs.borrow().progress_title.clone();
                let message = gs.borrow().progress_message.clone();
                let current = gs.borrow().progress_current;
                let total = gs.borrow().progress_total;
                let fraction = if total > 0 { current as f64 / total as f64 } else { 0.0 };

                let mut cell = progress_dialog_cell.borrow_mut();
                if let Some(ref dlg) = *cell {
                    // Update existing dialog
                    if let Some(child) = dlg.first_child() {
                        if let Some(vbox) = child.downcast_ref::<gtk::Box>() {
                            if let Some(title_lbl) = vbox.first_child().and_then(|w| w.downcast_ref::<gtk::Label>().cloned()) {
                                title_lbl.set_label(&title);
                            }
                            if let Some(msg_lbl) = vbox.first_child().and_then(|w| w.next_sibling()).and_then(|w| w.downcast_ref::<gtk::Label>().cloned()) {
                                msg_lbl.set_label(&message);
                            }
                            if let Some(bar) = vbox.first_child().and_then(|w| w.next_sibling()).and_then(|w| w.next_sibling()).and_then(|w| w.downcast_ref::<gtk::ProgressBar>().cloned()) {
                                bar.set_fraction(fraction);
                            }
                        }
                    }
                } else {
                    // Create new dialog
                    let dlg = adw::Dialog::builder()
                        .title(&title)
                        .content_width(400)
                        .content_height(160)
                        .can_close(false)
                        .build();

                    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
                    vbox.set_margin_top(16); vbox.set_margin_bottom(16);
                    vbox.set_margin_start(16); vbox.set_margin_end(16);

                    let title_lbl = gtk::Label::builder().label(&title).xalign(0.0).build();
                    title_lbl.add_css_class("heading");
                    vbox.append(&title_lbl);
                    let msg_lbl = gtk::Label::builder().label(&message).xalign(0.0).wrap(true).build();
                    msg_lbl.add_css_class("dim-label");
                    vbox.append(&msg_lbl);
                    let bar = gtk::ProgressBar::builder().fraction(fraction).hexpand(true).build();
                    vbox.append(&bar);

                    dlg.set_child(Some(&vbox));
                    dlg.present(Some(&w));
                    *cell = Some(dlg);
                }
            } else {
                let mut cell = progress_dialog_cell.borrow_mut();
                if let Some(dlg) = cell.take() {
                    dlg.close();
                }
            }

            // Banner (indeterminate text messages)
            let title = gs.borrow().message_dialog_title.clone();
            let active = !title.is_empty();
            pb.set_revealed(active && !progress_active);
            if active && !progress_active {
                pb.set_title(&title);
            }
            glib::ControlFlow::Continue
        });
    }

    // === Select button ===
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

    // === Add files (already in header) ===
    {
        let state = state.clone();
        let file_store = file_store.clone();
        let gs = gui_state.clone();
        add_files_btn.connect_clicked(move |_| {
            crate::connect::files::pick_files_and_add(&state, &file_store, &gs);
        });
    }

    // === Add folders (already in header) ===
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
        let file_store_c = file_store.clone();
        add_rule_btn.connect_clicked(move |_| {
            super::rule_editor::show_rule_editor(&window, &es, &state, &rule_store, &file_store_c, &gs, None);
        });
    }
    {
        let window = window.clone();
        let es = editor_state.clone();
        let state = state.clone();
        let rule_store = rule_store.clone();
        let gs = gui_state.clone();
        let file_store_c = file_store.clone();
        edit_rule_btn.connect_clicked(move |_| {
            let idx = state.borrow().rule_selected.iter().position(|x| *x).map(|i| i as i32).unwrap_or(-1);
            super::rule_editor::show_rule_editor(&window, &es, &state, &rule_store, &file_store_c, &gs, Some(idx));
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        let file_store_c = file_store.clone();
        remove_rule_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::remove_rule(&rule_store, &file_store_c, &state, &gs, -1);
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        let file_store_c = file_store.clone();
        rule_up_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::move_rule_up(&rule_store, &file_store_c, &state, &gs);
        });
    }
    {
        let rule_store = rule_store.clone();
        let state = state.clone();
        let gs = gui_state.clone();
        let file_store_c = file_store.clone();
        rule_down_btn.connect_clicked(move |_| {
            crate::connect::rules_ops::move_rule_down(&rule_store, &file_store_c, &state, &gs);
        });
    }
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
    {
        let state = state.clone();
        let rule_store = rule_store.clone();
        let gs = gui_state.clone();
        let window = window.clone();
        let file_store_c = file_store.clone();
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
            vbox.set_margin_top(12); vbox.set_margin_bottom(12);
            vbox.set_margin_start(12); vbox.set_margin_end(12);
            let list_box = gtk::ListBox::new();
            list_box.add_css_class("boxed-list");
            for (i, entry) in all.iter().enumerate() {
                let row = adw::ActionRow::builder().title(&entry.name).activatable(false).build();
                let load_btn = gtk::Button::with_label(&crate::fls!("rule_editor_load"));
                load_btn.add_css_class("flat"); load_btn.add_css_class("suggested-action");
                let st = state.clone(); let store2 = rule_store.clone(); let fs = file_store_c.clone();
                let gs2 = gs.clone(); let d = dialog.clone(); let idx = i as i32;
                load_btn.connect_clicked(move |_| {
                    crate::connect::rules_ops::load_rule_set(&store2, &fs, &st, &gs2, idx);
                    d.close();
                });
                row.add_suffix(&load_btn);
                let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
                del_btn.add_css_class("flat"); del_btn.add_css_class("destructive-action");
                let d2 = dialog.clone(); let idx = i as i32;
                del_btn.connect_clicked(move |_| { crate::connect::rules_ops::delete_rule_set(idx); d2.close(); });
                row.add_suffix(&del_btn);
                list_box.append(&row);
            }
            vbox.append(&list_box);
            let close_btn = gtk::Button::with_label(&crate::fls!("dialog_button_cancel"));
            close_btn.set_halign(gtk::Align::End);
            { let d = dialog.clone(); close_btn.connect_clicked(move |_| { d.close(); }); }
            vbox.append(&close_btn);
            dialog.set_child(Some(&vbox));
            dialog.present(Some(&window));
        });
    }

    gtk_app
}

fn show_preferences_dialog(window: &adw::ApplicationWindow, style_manager: &adw::StyleManager) {
    let dialog = adw::PreferencesDialog::new();

    let appearance_page = adw::PreferencesPage::new();
    appearance_page.set_icon_name(Some("preferences-desktop-appearance-symbolic"));
    appearance_page.set_title(&crate::fls!("menu_appearance"));
    let appearance_group = adw::PreferencesGroup::builder().title(&crate::fls!("menu_appearance")).build();
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

    let language_page = adw::PreferencesPage::new();
    language_page.set_icon_name(Some("preferences-desktop-locale-symbolic"));
    language_page.set_title(&crate::fls!("settings_language_label"));
    let language_group = adw::PreferencesGroup::builder().title(&crate::fls!("settings_language_label")).build();
    let saved_lang = crate::config::load_saved_language();
    let lang_combo = adw::ComboRow::builder()
        .title(&crate::fls!("settings_language_label"))
        .model(&gtk::StringList::new(
            &crate::language::LANGUAGES_ALL.iter().map(|l| l.combo_box_text).collect::<Vec<_>>()
        ))
        .selected(crate::language::LANGUAGES_ALL.iter().position(|l| l.combo_box_text == saved_lang).unwrap_or(0) as u32)
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
                confirm.connect_response(Some("restart"), move |_, _| { w2.close(); w2.application().unwrap().activate(); });
                confirm.present(Some(&w));
            }
        });
    }
    language_group.add(&lang_combo);
    language_page.add(&language_group);
    dialog.add(&language_page);
    dialog.present(Some(window));
}

fn show_select_popup(window: &adw::ApplicationWindow, file_store: &gio::ListStore, state: &SharedState, _gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder().heading(&crate::fls!("dialog_select")).body(&crate::fls!("dialog_select_body")).build();
    dialog.add_response("all", &crate::fls!("button_select_all"));
    dialog.add_response("none", &crate::fls!("button_unselect_all"));
    dialog.add_response("reverse", &crate::fls!("button_select_reverse"));
    dialog.add_response("custom", &crate::fls!("button_select_custom"));
    dialog.add_response("changed", &crate::fls!("button_select_changed"));
    dialog.add_response("unchanged", &crate::fls!("button_unselect_changed"));
    let store = file_store.clone();
    let st = state.clone();
    let w = window.clone();
    let gs = _gui_state.clone();
    dialog.connect_response(None, move |_, response| {
        match response {
            "all" => crate::connect::select::apply_select(&store, &st, SelectMode::SelectAll),
            "none" => crate::connect::select::apply_select(&store, &st, SelectMode::UnselectAll),
            "reverse" => crate::connect::select::apply_select(&store, &st, SelectMode::Reverse),
            "changed" => crate::connect::select::apply_select(&store, &st, SelectMode::SelectChanged),
            "unchanged" => crate::connect::select::apply_select(&store, &st, SelectMode::UnselectChanged),
            "custom" => { show_select_custom_dialog(&w, &store, &st, &gs); }
            _ => {}
        }
    });
    dialog.present(Some(window));
}

pub fn show_select_custom_dialog(window: &adw::ApplicationWindow, file_store: &gio::ListStore, state: &SharedState, _gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder().heading(&crate::fls!("dialog_select_custom_title")).body(&crate::fls!("dialog_select_custom_body")).build();
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content_box.set_margin_top(8); content_box.set_margin_bottom(8);
    content_box.set_margin_start(8); content_box.set_margin_end(8);
    let pattern_entry = gtk::Entry::builder().placeholder_text(&crate::fls!("dialog_select_custom_pattern")).hexpand(true).build();
    content_box.append(&pattern_entry);
    let include_dirs_check = gtk::CheckButton::with_label(&crate::fls!("dialog_select_custom_include_dirs"));
    include_dirs_check.set_active(true);
    content_box.append(&include_dirs_check);
    content_box.append(&gtk::Label::builder().label(&crate::fls!("ctrl_match_against")).xalign(0.0).build());
    let mode_combo = gtk::DropDown::from_strings(&[
        &crate::fls!("select_custom_path"), &crate::fls!("select_custom_current_path"),
        &crate::fls!("select_custom_future_path"), &crate::fls!("select_custom_path_current_name"),
        &crate::fls!("select_custom_path_future_name"), &crate::fls!("select_custom_directory_file"),
    ]);
    content_box.append(&mode_combo);
    let hint = gtk::Label::builder().label(&crate::fls!("select_custom_hint")).wrap(true).xalign(0.0).build();
    hint.add_css_class("dim-label");
    content_box.append(&hint);
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
        crate::connect::select::apply_select_custom(&store, &st, &pe.text(), idc.is_active(), mc.selected() as i32, true);
    });
    let store = file_store.clone();
    let st = state.clone();
    let pe = pattern_entry.clone();
    let idc = include_dirs_check.clone();
    let mc = mode_combo.clone();
    dialog.connect_response(Some("unselect"), move |_, _| {
        crate::connect::select::apply_select_custom(&store, &st, &pe.text(), idc.is_active(), mc.selected() as i32, false);
    });
    dialog.present(Some(window));
}

fn show_add_folders_dialog(window: &adw::ApplicationWindow, state: &SharedState, file_store: &gio::ListStore, gui_state: &SharedGuiState) {
    let dialog = adw::AlertDialog::builder()
        .heading(&crate::fls!("dialog_add_folders_title"))
        .body(&crate::fls!("dialog_add_folders_body"))
        .build();
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content_box.set_margin_top(8); content_box.set_margin_bottom(8);
    content_box.set_margin_start(8); content_box.set_margin_end(8);
    {
        let gs = gui_state.borrow();
        let paths = &gs.add_folder_picked_paths;
        let text = if paths.is_empty() { "No folders selected".to_string() } else { paths.join("\n") };
        content_box.append(&gtk::Label::builder().label(&text).xalign(0.0).wrap(true).build());
    }
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
        crate::connect::files::confirm_add_folders(&st, &store, &gs, scan_check.is_active(), ignore_check.is_active());
    });
    dialog.present(Some(window));
}

fn build_file_column_view(state: &SharedState, _window: &adw::ApplicationWindow) -> (gtk::ColumnView, gio::ListStore, gtk::MultiSelection) {
    let file_store = gio::ListStore::new::<FileRow>();
    let initial_selection = gtk::MultiSelection::new(Some(file_store.clone()));
    let column_view = gtk::ColumnView::new(Some(initial_selection.clone()));
    drop(initial_selection);
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_single_click_activate(false);
    let make_factory = |col_idx: usize| {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_xalign(0.0); label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_margin_start(6); label.set_margin_end(6);
            li.set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = li.item().and_downcast::<FileRow>().unwrap();
            let label = li.child().and_downcast::<gtk::Label>().unwrap();
            let text = match col_idx {
                0 => if row.is_dir() { "Dir" } else { "File" }.to_string(),
                1 => row.current_name(), 2 => row.future_name(), 3 => row.path(), _ => String::new(),
            };
            label.set_label(&text);
            label.remove_css_class("future-name-changed");
            if col_idx == 2 && text != row.current_name() {
                label.add_css_class("future-name-changed");
            }
        });
        factory
    };
    let type_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_type")), Some(make_factory(0)));
    type_col.set_fixed_width(50);
    type_col.set_resizable(true);
    type_col.set_sorter(Some(&gtk::CustomSorter::new(|a, b| a.downcast_ref::<FileRow>().unwrap().is_dir().cmp(&b.downcast_ref::<FileRow>().unwrap().is_dir()).into())));
    column_view.append_column(&type_col);
    let current_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_current_name")), Some(make_factory(1)));
    current_col.set_expand(true);
    current_col.set_resizable(true);
    current_col.set_sorter(Some(&gtk::CustomSorter::new(|a, b| natord::compare(&a.downcast_ref::<FileRow>().unwrap().current_name(), &b.downcast_ref::<FileRow>().unwrap().current_name()).into())));
    column_view.append_column(&current_col);
    let future_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_future_name")), Some(make_factory(2)));
    future_col.set_expand(true);
    future_col.set_resizable(true);
    future_col.set_sorter(Some(&gtk::CustomSorter::new(|a, b| natord::compare(&a.downcast_ref::<FileRow>().unwrap().future_name(), &b.downcast_ref::<FileRow>().unwrap().future_name()).into())));
    column_view.append_column(&future_col);
    let path_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_upper_column_path")), Some(make_factory(3)));
    path_col.set_expand(true);
    path_col.set_resizable(true);
    path_col.set_sorter(Some(&gtk::CustomSorter::new(|a, b| natord::compare(&a.downcast_ref::<FileRow>().unwrap().path(), &b.downcast_ref::<FileRow>().unwrap().path()).into())));
    column_view.append_column(&path_col);
    let sort_model = gtk::SortListModel::new(Some(file_store.clone()), column_view.sorter());
    let selection = gtk::MultiSelection::new(Some(sort_model.clone()));
    column_view.set_model(Some(&selection));
    state.borrow_mut().file_selection = Some(selection.clone());
    state.borrow_mut().file_sort_model = Some(sort_model);
    // Double-click to open file
    { let st = state.clone(); let gesture = gtk::GestureClick::new(); gesture.set_button(1);
        gesture.connect_pressed(move |_, n_press, _, _| { if n_press == 2 { let s = st.borrow(); if let Some(idx) = s.file_selected.iter().position(|x| *x) { if let Some(item) = s.files.get(idx) { let _ = open::that(&item.full_name); } } } }); column_view.add_controller(gesture); }
    // Right-click to open containing folder
    { let st = state.clone(); let gesture = gtk::GestureClick::new(); gesture.set_button(3);
        gesture.connect_released(move |_, _, _, _| { let s = st.borrow(); if let Some(idx) = s.file_selected.iter().position(|x| *x) { if let Some(item) = s.files.get(idx) { let _ = open::that(&item.path); } } }); column_view.add_controller(gesture); }
    (column_view, file_store, selection)
}

fn build_rule_column_view(selection: &gtk::MultiSelection, state: &SharedState, editor_state: &SharedEditorState, rule_store: &gio::ListStore, file_store: &gio::ListStore, gui_state: &SharedGuiState, window: &adw::ApplicationWindow) -> gtk::ColumnView {
    let column_view = gtk::ColumnView::new(Some(selection.clone()));
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_single_click_activate(false);
    let make_factory = |col_idx: usize| {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_xalign(0.0); label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_margin_start(6); label.set_margin_end(6);
            li.set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            let li = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = li.item().and_downcast::<RuleRow>().unwrap();
            let label = li.child().and_downcast::<gtk::Label>().unwrap();
            let text = match col_idx { 0 => row.rule_type_text(), 1 => row.usage_text(), 2 => row.description(), _ => String::new() };
            label.set_label(&text);
        });
        factory
    };
    let tool_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_tool_type")), Some(make_factory(0)));
    tool_col.set_expand(true); column_view.append_column(&tool_col);
    let usage_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_usage_name")), Some(make_factory(1)));
    usage_col.set_expand(true); column_view.append_column(&usage_col);
    let desc_col = gtk::ColumnViewColumn::new(Some(&crate::fls!("tree_view_bottom_description")), Some(make_factory(2)));
    desc_col.set_expand(true); column_view.append_column(&desc_col);
    // Double-click to edit rule
    { let st = state.clone(); let es = editor_state.clone(); let rs = rule_store.clone();
      let fs = file_store.clone(); let gs = gui_state.clone(); let w = window.clone(); let gesture = gtk::GestureClick::new(); gesture.set_button(1);
      gesture.connect_released(move |_, n_press, _, _| { if n_press >= 2 {
          let idx = st.borrow().rule_selected.iter().position(|x| *x).map(|i| i as i32).unwrap_or(0);
          super::rule_editor::show_rule_editor(&w, &es, &st, &rs, &fs, &gs, Some(idx));
      }}); column_view.add_controller(gesture); }
    column_view
}
