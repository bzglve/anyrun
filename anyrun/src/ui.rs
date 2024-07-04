use std::{cell::RefCell, io, rc::Rc, time::Duration};

use anyrun_interface::{HandleResult, Match};
use gtk::{
    gdk::Key,
    glib::{self, clone, SourceId},
    prelude::*,
};
use gtk_layer_shell::LayerShell;
use log::*;

use crate::{
    config::{style_names, Edge, PostRunAction, RelativeNum, RuntimeData},
    gmatch::GMatch,
    plugins::refresh_matches,
};

pub fn setup_main_window(
    app: &impl IsA<gtk::Application>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> Rc<gtk::ApplicationWindow> {
    let width = runtime_data
        .borrow()
        .config
        .width
        .to_val(runtime_data.borrow().geometry.width().try_into().unwrap());
    let height = runtime_data
        .borrow()
        .config
        .height
        .to_val(runtime_data.borrow().geometry.height().try_into().unwrap());

    info!("{} {}", width, height);

    let window = Rc::new(
        gtk::ApplicationWindow::builder()
            .application(app)
            .name(style_names::WINDOW)
            .default_width(width)
            .default_height(height)
            .build(),
    );

    setup_layer_shell(window.clone(), runtime_data.clone());

    let window_eck = gtk::EventControllerKey::new();
    connect_window_key_press_events(window.clone(), window_eck, window.clone());

    window.present();
    window
}

fn setup_layer_shell(window: Rc<impl GtkWindowExt>, runtime_data: Rc<RefCell<RuntimeData>>) {
    window.init_layer_shell();

    let config = &runtime_data.borrow().config;
    let geometry = runtime_data.borrow().geometry;
    let width = geometry.width().try_into().unwrap();
    let height = geometry.height().try_into().unwrap();

    for (i, edge) in config.edges.clone().into_iter().enumerate() {
        let margin = config
            .margin
            .get(i)
            .unwrap_or(&RelativeNum::default())
            .to_val(match edge {
                Edge::Left | Edge::Right => width,
                Edge::Top | Edge::Bottom => height,
            });
        window.set_anchor(edge.into(), true);
        window.set_margin(edge.into(), margin);
    }

    window.set_namespace("anyrun");

    if config.ignore_exclusive_zones {
        window.set_exclusive_zone(-1);
    }

    window.set_keyboard_mode(if config.steal_focus {
        gtk_layer_shell::KeyboardMode::Exclusive
    } else {
        gtk_layer_shell::KeyboardMode::OnDemand
    });

    window.set_layer(config.layer.into());
}

pub fn setup_entry(
    window: Rc<impl GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) -> Rc<gtk::SearchEntry> {
    let entry = Rc::new(
        gtk::SearchEntry::builder()
            .hexpand(true)
            .name(style_names::ENTRY)
            .placeholder_text("Search")
            .build(),
    );

    let entry_eck = gtk::EventControllerKey::new();
    connect_entry_key_press_events(entry.clone(), entry_eck, window.clone());

    let debounce_timeout: Rc<RefCell<Option<SourceId>>> = Rc::new(RefCell::new(None));
    entry.connect_changed(clone!(@strong debounce_timeout => move |e| {
        if let Some(timeout_id) = debounce_timeout.borrow_mut().take() {
            timeout_id.remove();
        }

        runtime_data.borrow_mut().exclusive = None;
        *debounce_timeout.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(runtime_data.borrow().config.smooth_input_time),
            glib::
            clone!(@weak e, @weak runtime_data, @strong debounce_timeout => move || {
                *debounce_timeout.borrow_mut() = None;
                refresh_matches(&e.text(), runtime_data.clone());
            }),
        ));
    }));

    entry
}

pub fn setup_activation(
    entry: Rc<gtk::SearchEntry>,
    main_list: Rc<gtk::ListBox>,
    window: Rc<impl GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
) {
    entry.connect_activate(clone!(@weak main_list, @weak window, @weak runtime_data =>
        move |e| {
        if let Some(row) = main_list.selected_row() {
            handle_selection_activation(
                row.index().try_into().unwrap(),
                window.clone(),
                runtime_data.clone(),
                |_| refresh_matches(&e.text(), runtime_data.clone()),
            )
        }
    }));

    main_list.connect_row_activated(clone!(@weak window, @weak runtime_data, @weak entry =>
        move |_, row| {
        handle_selection_activation(
            row.index().try_into().unwrap(),
            window.clone(),
            runtime_data.clone(),
            |_| refresh_matches(&entry.text(), runtime_data.clone()),
        )
    }));
}

fn connect_key_press_events<F>(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    handler: F,
) where
    F: Fn(Key) -> glib::Propagation + 'static,
{
    widget.add_controller(event_controller_key.clone());
    event_controller_key.connect_key_pressed(move |_, keyval, _, _| handler(keyval));
}

fn connect_window_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    window: Rc<impl GtkWindowExt>,
) {
    connect_key_press_events(widget, event_controller_key, move |keyval| match keyval {
        Key::Escape => {
            window.close();
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
}

fn connect_entry_key_press_events(
    widget: Rc<impl WidgetExt>,
    event_controller_key: gtk::EventControllerKey,
    window: Rc<impl GtkWindowExt>,
) {
    connect_key_press_events(
        widget.clone(),
        event_controller_key,
        move |keyval| match keyval {
            Key::Escape => {
                window.close();
                glib::Propagation::Stop
            }
            Key::Down | Key::Up => {
                widget.emit_move_focus(if keyval == Key::Down {
                    gtk::DirectionType::TabForward
                } else {
                    gtk::DirectionType::TabBackward
                });
                glib::Propagation::Proceed
            }
            _ => glib::Propagation::Proceed,
        },
    );
}

fn handle_selection_activation<F>(
    row_id: usize,
    window: Rc<impl GtkWindowExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    mut on_refresh: F,
) where
    F: FnMut(bool),
{
    let gmatch = runtime_data
        .borrow()
        .list_store
        .item(row_id.try_into().unwrap())
        .unwrap_or_else(|| panic!("Failed to get list_store item at {} position", row_id))
        .downcast::<GMatch>()
        .expect("Failed to downcast Object to MatchRow");

    let rmatch: Match = gmatch.clone().into();
    let plugin = *runtime_data
        .borrow()
        .plugins
        .get(gmatch.get_plugin_id() as usize)
        .expect("Can't get plugin");

    match plugin.handle_selection()(rmatch) {
        HandleResult::Close => window.close(),
        HandleResult::Refresh(exclusive) => {
            runtime_data.borrow_mut().exclusive = if exclusive { Some(plugin) } else { None };
            on_refresh(exclusive);
        }
        HandleResult::Copy(bytes) => {
            runtime_data.borrow_mut().post_run_action = PostRunAction::Copy(bytes.into());
            window.close();
        }
        HandleResult::Stdout(bytes) => {
            if let Err(why) = io::Write::write_all(&mut io::stdout().lock(), &bytes) {
                error!("Error outputting content to stdout: {}", why);
            }
            window.close();
        }
    }
}

pub fn configure_main_window(
    window: Rc<impl WidgetExt + GtkWindowExt + NativeExt>,
    runtime_data: Rc<RefCell<RuntimeData>>,
    entry: Rc<impl WidgetExt>,
    main_list: Rc<impl WidgetExt>,
) {
    let runtime_data = runtime_data.borrow();

    let main_vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Fill)
        .valign(gtk::Align::Fill)
        .name(style_names::MAIN)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .spacing(12)
        .build();

    if !runtime_data.error_label.is_empty() {
        main_vbox.append(
            &gtk::Label::builder()
                .label(format!(
                    r#"<span foreground="red">{}</span>"#,
                    runtime_data.error_label
                ))
                .use_markup(true)
                .build(),
        );
    }

    let scroll_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .focusable(false)
        .build();

    scroll_window.set_child(Some(&*main_list));

    if runtime_data.config.bottom_entry {
        main_vbox.append(&scroll_window);
        main_vbox.append(&*entry);
    } else {
        main_vbox.append(&*entry);
        main_vbox.append(&scroll_window);
    }

    window.set_child(Some(&main_vbox));
    entry.grab_focus();
}
