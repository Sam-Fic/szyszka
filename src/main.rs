#![windows_subsystem = "windows"]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;

mod cli_arguments;
mod config;
mod connect;
mod files;
mod language;
mod localizer;
mod logger;
mod rule;
mod state;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::prelude::*;

use crate::cli_arguments::{handle_help_version, parse_cli_paths};
use crate::config::{load_dark_theme_config_or_create, load_saved_language};
use crate::connect::files::add_cli_paths;
use crate::connect::sync::{sync_files, sync_outdated, sync_rules};
use crate::connect::translations::apply_translations;
use crate::language::apply_language;
use crate::state::new_shared;
use crate::ui::state_ui::{new_shared_editor_state, new_shared_gui_state};

fn main() -> glib::ExitCode {
    let cli_args: Vec<String> = std::env::args().collect();
    handle_help_version(&cli_args);
    let cli_paths = parse_cli_paths(&cli_args);

    let saved_language = load_saved_language();
    apply_language(&saved_language);

    let _ = gtk::init();

    gio::resources_register_include!("com.github.samfic.szyszka.gresource").expect("Failed to register bundled resources");

    let app = adw::Application::builder().application_id("com.github.samfic.szyszka").build();

    app.connect_startup(|_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let theme = gtk::IconTheme::for_display(&display);
            theme.add_resource_path("/com/github/samfic/szyszka");
        }
    });

    let current_window: Rc<RefCell<Option<adw::ApplicationWindow>>> = Rc::new(RefCell::new(None));

    let state = new_shared();
    let editor_state = new_shared_editor_state();
    let gui_state = new_shared_gui_state();
    let translations = Rc::new(RefCell::new(apply_translations()));

    let is_dark = load_dark_theme_config_or_create();
    if is_dark {
        app.style_manager().set_color_scheme(libadwaita::ColorScheme::ForceDark);
    } else {
        app.style_manager().set_color_scheme(libadwaita::ColorScheme::ForceLight);
    }

    let state_clone = state.clone();
    let editor_state_clone = editor_state.clone();
    let gui_state_clone = gui_state.clone();
    let translations_clone = translations.clone();
    let cli_paths_clone = cli_paths.clone();
    let current_window_clone = current_window.clone();

    app.connect_activate(move |app| {
        // Destroy old window if exists
        if let Some(old_window) = current_window_clone.borrow_mut().take() {
            old_window.close();
        }

        // Re-apply language to refresh translations
        let lang = load_saved_language();
        apply_language(&lang);

        let gtk_app = ui::main_window::build_gtk_app(app, state_clone.clone(), editor_state_clone.clone(), gui_state_clone.clone(), translations_clone.clone());

        *current_window_clone.borrow_mut() = Some(gtk_app.window.clone());

        crate::connect::rules_ops::refresh_rule_sets(&gui_state_clone);

        sync_files(&gtk_app.file_store, &state_clone);
        sync_rules(&gtk_app.rule_store, &state_clone);
        sync_outdated(&gui_state_clone, &state_clone);

        add_cli_paths(&state_clone, &gtk_app.file_store, &gui_state_clone, cli_paths_clone.clone());

        gtk_app.window.present();
    });

    app.run()
}
