mod config;
mod gmatch;
mod plugins;
mod ui;
mod utils;

use std::{cell::RefCell, rc::Rc};

use clap::Parser;
use gmatch::GMatch;
use gtk::{
    gdk, gio,
    glib::{self, clone},
    prelude::*,
};
#[allow(unused_imports)]
use log::*;

use config::*;
use plugins::*;
use ui::*;
use utils::*;

fn main() -> Result<glib::ExitCode, glib::Error> {
    env_logger::init();
    gtk::init().expect("Failed to initialize GTK.");

    let app = gtk::Application::new(Some(APP_ID), Default::default());
    app.register(gio::Cancellable::NONE)?;

    if app.is_remote() {
        return Ok(glib::ExitCode::SUCCESS);
    }

    let app_state = gio::Settings::new(APP_ID);

    let args = Args::parse();
    let config_dir = determine_config_dir(&args.config_dir);
    let (mut config, error_label) = load_config(&config_dir);
    config.merge_opt(args.config);

    let display = gdk::Display::default().expect("No display found");
    let monitor = display
        .monitors()
        .into_iter()
        .filter_map(|m| m.ok())
        .peekable()
        .peek()
        .expect("No monitor found")
        .clone()
        .downcast::<gdk::Monitor>()
        .expect("Can't downcast Object to Monitor");
    let geometry = monitor.geometry();

    let list_store = gio::ListStore::builder()
        .item_type(GMatch::static_type())
        .build();

    let plugins = config
        .plugins
        .iter()
        .map(|filename| load_plugin(filename, &config_dir))
        .collect();

    let runtime_data = Rc::new(RefCell::new(RuntimeData {
        exclusive: None,
        post_run_action: PostRunAction::None,
        config,
        error_label,
        config_dir,
        geometry,
        list_store,
        plugins,
        app_state,
    }));

    app.connect_activate(
        clone!(@weak runtime_data => move |app| activate(app, runtime_data.clone())),
    );
    let exit_code = app.run_with_args::<String>(&[]);

    handle_post_run_action(runtime_data);

    Ok(exit_code)
}

fn activate(app: &impl IsA<gtk::Application>, runtime_data: Rc<RefCell<RuntimeData>>) {
    load_custom_css(runtime_data.clone());

    let main_list = Rc::new(
        gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Browse)
            .name(style_names::MAIN)
            .build(),
    );

    let list_store = runtime_data.clone().borrow().list_store.clone();

    main_list.bind_model(
        Some(&list_store),
        clone!(@strong runtime_data => move |match_row| {
            match_row
                .clone()
                .downcast::<GMatch>()
                .expect("Can't downcast glib::Object to GMatch")
                .to_widget(runtime_data.clone())
        }),
    );

    let window = setup_main_window(app, runtime_data.clone());

    let entry = setup_entry(window.clone(), runtime_data.clone());

    if runtime_data.borrow().config.save_entry_state {
        let app_state = runtime_data.borrow().app_state.clone();
        entry.set_text(&app_state.string("entry-state"));
        app_state.bind("entry-state", &*entry, "text").build();
    }

    list_store.connect_items_changed(
        clone!(@weak entry, @strong main_list, @weak runtime_data => move |_, _, _, _| {
            main_list.select_row(main_list.row_at_index(0).as_ref())
        }),
    );

    setup_activation(
        entry.clone(),
        main_list.clone(),
        window.clone(),
        runtime_data.clone(),
    );

    if runtime_data.borrow().config.show_results_immediately {
        refresh_matches(&entry.text(), runtime_data.clone());
    }

    configure_main_window(
        window.clone(),
        runtime_data.clone(),
        entry.clone(),
        main_list.clone(),
    );

    window.present();
}
