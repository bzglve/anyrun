use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use gtk::{gdk, gio, prelude::InputStreamExtManual};
use log::*;
use nix::unistd;
use wl_clipboard_rs::copy;

use crate::{
    config::{style_names, PostRunAction, RuntimeData},
    SOCKET_BUF_SIZE,
};

fn serve_copy_requests(bytes: &[u8]) {
    let mut opts = copy::Options::new();
    opts.foreground(true);
    opts.copy(
        copy::Source::Bytes(bytes.to_vec().into_boxed_slice()),
        copy::MimeType::Autodetect,
    )
    .expect("Failed to serve copy bytes");
}

pub fn handle_post_run_action(runtime_data: Rc<RefCell<RuntimeData>>) {
    if let PostRunAction::Copy(bytes) = &runtime_data.borrow().post_run_action {
        match unsafe { unistd::fork() } {
            Ok(unistd::ForkResult::Parent { .. }) => {
                info!("Child spawned to serve copy requests.");
            }
            Ok(unistd::ForkResult::Child) => {
                serve_copy_requests(bytes);
            }
            Err(why) => {
                error!("Failed to fork for copy sharing: {}", why);
            }
        }
    }
}

pub fn load_custom_css(runtime_data: Rc<RefCell<RuntimeData>>) {
    let config_dir = &runtime_data.borrow().config_dir;
    let css_path = config_dir.join("style.css");

    if fs::metadata(&css_path).is_ok() {
        info!("Applying custom CSS from {:?}", css_path);
        let provider = gtk::CssProvider::new();
        provider.load_from_path(css_path);

        let display = gdk::Display::default().expect("Failed to get GDK display for CSS provider!");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

pub fn build_label(name: &str, use_markup: bool, label: &str) -> gtk::Label {
    gtk::Label::builder()
        .name(name)
        .wrap(true)
        .xalign(0.0)
        .use_markup(use_markup)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .label(label)
        .build()
}

pub fn build_image(icon: &str) -> gtk::Image {
    let mut match_image = gtk::Image::builder()
        .name(style_names::MATCH)
        .pixel_size(32);

    let path = PathBuf::from(icon);

    match_image = if path.is_absolute() {
        match_image.file(path.to_string_lossy())
    } else {
        match_image.icon_name(icon)
    };
    match_image.build()
}

pub fn read_from_stream(stream: &impl InputStreamExtManual) -> String {
    let mut buf = [0; SOCKET_BUF_SIZE];
    let _count = stream.read(&mut buf, gio::Cancellable::NONE);
    String::from_utf8(buf.to_vec())
        .expect("Can't get string from bytes array")
        .trim_matches(char::from(0))
        .to_owned()
}
