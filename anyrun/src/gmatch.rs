use abi_stable::{
    std_types::{ROption, RString},
    traits::IntoReprRust,
};
use anyrun_interface::Match as RMatch;
use gtk::{
    gio::prelude::*,
    glib::{self, subclass::prelude::*, ParamSpec},
    prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    config::{style_names, RuntimeData},
    utils::{build_image, build_label},
};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct GMatch {
        pub title: RefCell<String>,
        pub description: RefCell<Option<String>>,
        pub use_pango: Cell<bool>,
        pub icon: RefCell<Option<String>>,
        pub id: Cell<u64>,
        // workarond to get something like `Option<u64>` for id with glib because I couldn't find some
        id_some: Cell<bool>,
        pub plugin_id: Cell<u64>,
        pub first: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GMatch {
        const NAME: &'static str = "GMatch";

        type Type = super::GMatch;
    }

    impl ObjectImpl for GMatch {
        fn properties() -> &'static [ParamSpec] {
            use std::sync::OnceLock;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("title").build(),
                    glib::ParamSpecString::builder("description").build(),
                    glib::ParamSpecBoolean::builder("use-pango").build(),
                    glib::ParamSpecString::builder("icon").build(),
                    glib::ParamSpecUInt64::builder("id").build(),
                    glib::ParamSpecBoolean::builder("id-some").build(),
                    glib::ParamSpecUInt64::builder("plugin-id").build(),
                    glib::ParamSpecBoolean::builder("first").build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "title" => {
                    let title = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.title.replace(title);
                }
                "description" => {
                    let description = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.description.replace(description);
                }
                "use-pango" => {
                    let use_pango = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.use_pango.replace(use_pango);
                }
                "icon" => {
                    let icon = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.icon.replace(icon);
                }
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "id-some" => {
                    let id_some = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id_some.replace(id_some);
                }
                "plugin-id" => {
                    let plugin_id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.plugin_id.replace(plugin_id);
                }
                "first" => {
                    let first = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.first.replace(first);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "title" => self.title.borrow().to_value(),
                "description" => self.description.borrow().to_value(),
                "use-pango" => self.use_pango.get().to_value(),
                "icon" => self.icon.borrow().to_value(),
                "id" => self.id.get().to_value(),
                "id-some" => self.id_some.get().to_value(),
                "plugin-id" => self.plugin_id.get().to_value(),
                "first" => self.first.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed()
        }
    }
}

glib::wrapper! {
    pub struct GMatch(ObjectSubclass<imp::GMatch>);
}

impl GMatch {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn get_title(&self) -> String {
        self.property("title")
    }

    pub fn set_title(&self, value: String) {
        self.set_property("title", value)
    }

    pub fn get_description(&self) -> Option<String> {
        self.property("description")
    }

    pub fn set_description(&self, value: Option<String>) {
        self.set_property("description", value)
    }

    pub fn get_use_pango(&self) -> bool {
        self.property("use-pango")
    }

    pub fn set_use_pango(&self, value: bool) {
        self.set_property("use-pango", value)
    }

    pub fn get_icon(&self) -> Option<String> {
        self.property("icon")
    }

    pub fn set_icon(&self, value: Option<String>) {
        self.set_property("icon", value)
    }

    pub fn get_id(&self) -> Option<u64> {
        let id = self.property("id");
        let id_some = self.property("id-some");

        if id_some {
            return Some(id);
        }
        None
    }

    pub fn set_id(&self, value: Option<u64>) {
        if let Some(value) = value {
            self.set_property("id", value);
            self.set_property("id-some", true);
        } else {
            self.set_property("id", 0u64);
            self.set_property("id-some", false);
        }
    }

    pub fn get_plugin_id(&self) -> u64 {
        self.property("plugin-id")
    }

    pub fn set_plugin_id(&self, value: u64) {
        self.set_property("plugin-id", value)
    }

    pub fn get_first(&self) -> bool {
        self.property("first")
    }

    pub fn set_first(&self, value: bool) {
        self.set_property("first", value);
    }

    pub fn to_widget(&self, runtime_data: Rc<RefCell<RuntimeData>>) -> gtk::Widget {
        let runtime_data = runtime_data.borrow();
        let plugin = runtime_data
            .plugins
            .get(self.get_plugin_id() as usize)
            .expect("Can't get plugin by id");

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(36)
            .spacing(4)
            .build();

        let plugin_info_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            // .halign(gtk::Align::Center)
            .width_request(200)
            .spacing(12)
            .build();

        let plugin_info = plugin.info()();

        let plugin_icon = build_image(&plugin_info.icon);
        plugin_icon.set_margin_start(4);
        plugin_icon.set_margin_end(8);
        plugin_info_box.append(&plugin_icon);

        plugin_icon.set_visible(!runtime_data.config.hide_plugins_icons && self.get_first());

        let plugin_label = gtk::Label::builder()
            .label(if self.get_first() {
                &plugin_info.name
            } else {
                ""
            })
            .build();

        plugin_info_box.append(&plugin_label);

        plugin_info_box.set_visible(!runtime_data.config.hide_plugin_info);

        hbox.append(&plugin_info_box);

        let match_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .build();

        if !runtime_data.config.hide_match_icons {
            if let Some(icon) = self.get_icon() {
                match_box.append(&build_image(&icon));
            }
        }

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();

        vbox.append(&build_label(
            style_names::MATCH_TITLE,
            self.get_use_pango(),
            &self.get_title(),
        ));

        if let Some(desc) = self.get_description() {
            vbox.append(&build_label(
                style_names::MATCH_DESC,
                self.get_use_pango(),
                &desc,
            ));
        }

        match_box.append(&vbox);
        hbox.append(&match_box);

        hbox.into()
    }
}

impl Default for GMatch {
    fn default() -> Self {
        Self::new()
    }
}

// workaround to get some representasion because there is already `Debug` for `glib::Object`
impl std::fmt::Display for GMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GMatch")
            .field("title", &self.get_title())
            .field("description", &self.get_description())
            .field("use_pango", &self.get_use_pango())
            .field("icon", &self.get_icon())
            .field("id", &self.get_id())
            .field("plugin_id", &self.get_plugin_id())
            .field("first", &self.get_first())
            .finish()
    }
}

impl From<RMatch> for GMatch {
    fn from(value: RMatch) -> Self {
        fn from_ropt_to_opt(value: ROption<RString>) -> Option<String> {
            if let ROption::RSome(s) = value {
                Some(s.to_string())
            } else {
                None
            }
        }

        let item = Self::new();

        item.set_title(value.title.into());
        item.set_description(from_ropt_to_opt(value.description));
        item.set_use_pango(value.use_pango);
        item.set_icon(from_ropt_to_opt(value.icon));
        item.set_id(value.id.into_rust());

        item.set_plugin_id(0);

        item.set_first(true);

        item
    }
}

impl From<GMatch> for RMatch {
    fn from(val: GMatch) -> Self {
        fn from_opt_to_ropt(value: Option<String>) -> ROption<RString> {
            if let Some(s) = value {
                ROption::RSome(s.into())
            } else {
                ROption::RNone
            }
        }

        RMatch {
            title: val.get_title().into(),
            description: from_opt_to_ropt(val.get_description()),
            use_pango: val.get_use_pango(),
            icon: from_opt_to_ropt(val.get_icon()),
            id: val.get_id().into(),
        }
    }
}
