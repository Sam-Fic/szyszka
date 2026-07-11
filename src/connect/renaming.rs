use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{mpsc, Arc};

use adw::prelude::*;

use crate::connect::progress::{hide_overlay, show_overlay};
use crate::files::CHARACTER;
use crate::fls;
use crate::state::SharedState;
use crate::ui::dialogs;
use crate::ui::state_ui::SharedGuiState;

struct RenameResult {
    properly_renamed: u32,
    ignored: u32,
    failed: Vec<(String, String, String)>,
}

pub fn start_renaming_request(window: &adw::ApplicationWindow, state: &SharedState, gui_state: SharedGuiState) {
    let state_ref = state.borrow();

    // Ignore re-entrant requests while an async operation is running.
    if state_ref.async_active {
        return;
    }

    if state_ref.files.is_empty() {
        dialogs::show_message_dialog(window, &fls!("renaming_missing_files"), &fls!("renaming_require_missing_files"));
        return;
    }
    if state_ref.rules.rules.is_empty() {
        dialogs::show_message_dialog(window, &fls!("renaming_missing_rules"), &fls!("renaming_require_missing_rules"));
        return;
    }

    if !state_ref.rules.updated {
        let dialog = adw::AlertDialog::builder()
            .heading(fls!("dialog_outdated_results"))
            .body(fls!("renaming_some_records_not_updated"))
            .build();
        dialog.add_response("cancel", &crate::fls!("dialog_button_cancel"));
        dialog.add_response("proceed", &crate::fls!("dialog_button_ok"));
        dialog.set_response_appearance("proceed", adw::ResponseAppearance::Suggested);
        let window_clone = window.clone();
        let state_clone = state.clone();
        let gui_clone = gui_state.clone();
        dialog.connect_response(Some("proceed"), move |_, _| {
            let count = state_clone.borrow().files.len() as i32;
            dialogs::show_confirm_dialog(&window_clone, state_clone.clone(), gui_clone.clone(), count);
        });
        dialog.present(Some(window));
    } else {
        let count = state_ref.files.len() as i32;
        drop(state_ref);
        dialogs::show_confirm_dialog(window, state.clone(), gui_state, count);
    }
}

pub fn perform_renaming(window: &adw::ApplicationWindow, state: &SharedState, gui_state: &SharedGuiState) {
    let mut file_renames: Vec<(String, String)> = Vec::new();
    let mut folder_renames: BTreeMap<usize, Vec<(String, String)>> = BTreeMap::new();

    {
        let state_ref = state.borrow();
        for file in &state_ref.files {
            let old_name = format!("{}{}{}", file.path, CHARACTER, file.name);
            let new_name = format!("{}{}{}", file.path, CHARACTER, file.future_name);
            if file.is_dir {
                let depth = old_name.matches(CHARACTER).count();
                folder_renames.entry(depth).or_default().push((old_name, new_name));
            } else {
                file_renames.push((old_name, new_name));
            }
        }
    }

    let total = file_renames.len() + folder_renames.values().map(|v| v.len()).sum::<usize>();
    log::info!("Renaming {total} items");

    // Show an inline, non-modal progress banner instead of a blocking dialog.
    // This matches the existing "Adding files…" / "Scanning…" flow and follows
    // the GNOME HIG (in-window progress rather than a modal popup that locks
    // the whole window).
    show_overlay(gui_state, &fls!("dialog_loading"), "", true);

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_w = counter.clone();
    let (tx, rx) = mpsc::channel::<RenameResult>();

    let dest_exists_msg = fls!("renaming_destination_file_exists");

    std::thread::spawn(move || {
        let mut failed: Vec<(String, String, String)> = Vec::new();
        let mut properly_renamed: u32 = 0;
        let mut ignored: u32 = 0;

        for (old_name, new_name) in file_renames {
            rename_one(&old_name, &new_name, &mut ignored, &mut properly_renamed, &mut failed, &dest_exists_msg);
            counter_w.fetch_add(1, AtomicOrdering::Relaxed);
        }
        for (_depth, vec) in folder_renames.iter().rev() {
            for (old_name, new_name) in vec {
                rename_one(old_name, new_name, &mut ignored, &mut properly_renamed, &mut failed, &dest_exists_msg);
                counter_w.fetch_add(1, AtomicOrdering::Relaxed);
            }
        }

        let _ = tx.send(RenameResult {
            properly_renamed,
            ignored,
            failed,
        });
    });

    let counter_p = counter;
    let state_clone = state.clone();
    let window_clone = window.clone();
    let gui_clone = gui_state.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(60), move || {
        let cur = counter_p.load(AtomicOrdering::Relaxed);
        log::debug!("Rename progress: {cur}/{total}");
        // Live count in the banner title gives determinate feedback inline.
        let title = if total > 0 {
            format!("{} {}/{}", fls!("dialog_loading"), cur, total)
        } else {
            fls!("dialog_loading")
        };
        gui_clone.borrow_mut().message_dialog_title = title;

        match rx.try_recv() {
            Ok(result) => {
                hide_overlay(&gui_clone);
                finalize_rename(&window_clone, &state_clone, &result);
                state_clone.borrow_mut().async_active = false;
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                hide_overlay(&gui_clone);
                state_clone.borrow_mut().async_active = false;
                glib::ControlFlow::Break
            }
        }
    });
    state.borrow_mut().async_active = true;
}

pub fn perform_renaming_gtk(window: &adw::ApplicationWindow, state: &SharedState, gui_state: &SharedGuiState) {
    perform_renaming(window, state, gui_state);
}

fn finalize_rename(window: &adw::ApplicationWindow, state: &SharedState, result: &RenameResult) {
    {
        let mut state_mut = state.borrow_mut();
        state_mut.files.clear();
        state_mut.file_selected.clear();
        state_mut.result_entries.files.clear();
    }

    {
        let text_err = fls!("renaming_error");
        let lines: Vec<String> = result.failed.iter().map(|(old, new, err)| format!("{old} -> {new}, {text_err}: {err}")).collect();
        state.borrow_mut().failed_renames = lines;
    }

    let failed_total = state.borrow().failed_renames.len();

    // Build results dialog
    let dialog = adw::AlertDialog::builder().heading(fls!("dialog_results_of_renaming")).build();

    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);
    content_box.set_margin_start(8);
    content_box.set_margin_end(8);

    content_box.append(&gtk::Label::builder().label(format!("Properly renamed: {}", result.properly_renamed)).xalign(0.0).build());
    content_box.append(&gtk::Label::builder().label(format!("Ignored: {}", result.ignored)).xalign(0.0).build());

    if failed_total > 0 {
        let err_label = gtk::Label::builder().label(format!("Errors: {failed_total}")).xalign(0.0).build();
        err_label.add_css_class("error");
        content_box.append(&err_label);

        // Error list with pagination
        let page_size = 500usize;
        let total_pages = failed_total.div_ceil(page_size);
        let current_page = std::rc::Rc::new(std::cell::Cell::new(0usize));

        let error_text_label = gtk::Label::builder().xalign(0.0).wrap(true).selectable(true).vexpand(true).build();
        error_text_label.add_css_class("card");
        error_text_label.set_margin_top(4);
        error_text_label.set_margin_bottom(4);

        // Load initial page
        {
            let lines = &state.borrow().failed_renames;
            let end = page_size.min(failed_total);
            error_text_label.set_label(&lines[..end].join("\n"));
        }

        let scroll = gtk::ScrolledWindow::builder().child(&error_text_label).min_content_height(200).vexpand(true).build();
        content_box.append(&scroll);

        // Pagination controls
        if total_pages > 1 {
            let page_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            page_box.set_halign(gtk::Align::Center);

            let prev_btn = gtk::Button::from_icon_name("go-previous-symbolic");
            prev_btn.set_sensitive(false);
            let page_label = gtk::Label::new(Some(&format!("1 / {total_pages}")));
            let next_btn = gtk::Button::from_icon_name("go-next-symbolic");

            let st = state.clone();
            let lbl = error_text_label;
            let plbl = page_label.clone();
            let cp = current_page.clone();
            let prev = prev_btn.clone();
            let nxt = next_btn.clone();
            let tp = total_pages;

            let update_page = move |page: usize| {
                cp.set(page);
                let lines = &st.borrow().failed_renames;
                let start = page * page_size;
                let end = (start + page_size).min(failed_total);
                lbl.set_label(&lines[start..end].join("\n"));
                plbl.set_label(&format!("{} / {}", page + 1, tp));
                prev.set_sensitive(page > 0);
                nxt.set_sensitive(page < tp - 1);
            };

            {
                let cp2 = current_page.clone();
                let update = update_page.clone();
                prev_btn.connect_clicked(move |_| {
                    let cur = cp2.get();
                    if cur > 0 {
                        update(cur - 1);
                    }
                });
            }
            {
                let cp2 = current_page;
                let update = update_page;
                next_btn.connect_clicked(move |_| {
                    let cur = cp2.get();
                    if cur < tp - 1 {
                        update(cur + 1);
                    }
                });
            }

            page_box.append(&prev_btn);
            page_box.append(&page_label);
            page_box.append(&next_btn);
            content_box.append(&page_box);
        }

        // Copy errors button
        let copy_btn = {
            let btn = gtk::Button::new();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            content.set_halign(gtk::Align::Center);
            content.append(&gtk::Image::from_icon_name("edit-copy-symbolic"));
            content.append(&gtk::Label::new(Some(&crate::fls!("dialog_copy_all_errors"))));
            btn.set_child(Some(&content));
            btn
        };
        copy_btn.set_halign(gtk::Align::Start);
        let st = state.clone();
        copy_btn.connect_clicked(move |_| {
            crate::connect::renaming::copy_all_errors(&st);
        });
        content_box.append(&copy_btn);
    }

    dialog.set_extra_child(Some(&content_box));
    dialog.add_response("ok", &crate::fls!("dialog_button_ok"));
    dialog.present(Some(window));
}

pub fn copy_all_errors(state: &SharedState) {
    let text = state.borrow().failed_renames.join("\n");
    if text.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        use arboard::Clipboard;
        #[cfg(target_os = "linux")]
        use arboard::SetExtLinux as _;
        match Clipboard::new() {
            Ok(mut ctx) => {
                #[cfg(target_os = "linux")]
                let result = ctx.set().wait().text(text);
                #[cfg(not(target_os = "linux"))]
                let result = ctx.set_text(text);
                if let Err(e) = result {
                    log::warn!("Failed to copy errors to clipboard: {e}");
                }
            }
            Err(e) => log::warn!("Failed to create clipboard context: {e}"),
        }
    });
}

fn rename_one(old_name: &str, new_name: &str, ignored: &mut u32, properly_renamed: &mut u32, failed_renames: &mut Vec<(String, String, String)>, dest_exists_msg: &str) {
    if new_name == old_name {
        *ignored += 1;
    } else if Path::new(new_name).exists() {
        failed_renames.push((old_name.to_string(), new_name.to_string(), dest_exists_msg.to_string()));
    } else if let Err(e) = fs::rename(old_name, new_name) {
        failed_renames.push((old_name.to_string(), new_name.to_string(), e.to_string()));
    } else {
        *properly_renamed += 1;
    }
}
