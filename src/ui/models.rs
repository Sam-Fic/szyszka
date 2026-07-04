use glib::prelude::*;
use glib::subclass::prelude::*;

mod imp {
    use super::*;
    use glib::Properties;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::FileRow)]
    pub struct FileRow {
        #[property(get, set)]
        pub selected: RefCell<bool>,
        #[property(get, set)]
        pub is_dir: RefCell<bool>,
        #[property(get, set)]
        pub current_name: RefCell<String>,
        #[property(get, set)]
        pub future_name: RefCell<String>,
        #[property(get, set)]
        pub path: RefCell<String>,
        #[property(get, set)]
        pub size_text: RefCell<String>,
        #[property(get, set)]
        pub date_text: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileRow {
        const NAME: &'static str = "SzyszkaFileRow";
        type Type = super::FileRow;
    }

    impl ObjectImpl for FileRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }
        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub struct FileRow(ObjectSubclass<imp::FileRow>);
}

impl FileRow {
    pub fn new(selected: bool, is_dir: bool, current_name: &str, future_name: &str, path: &str, size_text: &str, date_text: &str) -> Self {
        glib::Object::builder()
            .property("selected", selected)
            .property("is-dir", is_dir)
            .property("current-name", current_name)
            .property("future-name", future_name)
            .property("path", path)
            .property("size-text", size_text)
            .property("date-text", date_text)
            .build()
    }
}

mod imp_rule {
    use super::*;
    use glib::Properties;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::RuleRow)]
    pub struct RuleRow {
        #[property(get, set)]
        pub selected: RefCell<bool>,
        #[property(get, set)]
        pub rule_type_text: RefCell<String>,
        #[property(get, set)]
        pub usage_text: RefCell<String>,
        #[property(get, set)]
        pub description: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RuleRow {
        const NAME: &'static str = "SzyszkaRuleRow";
        type Type = super::RuleRow;
    }

    impl ObjectImpl for RuleRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }
        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub struct RuleRow(ObjectSubclass<imp_rule::RuleRow>);
}

impl RuleRow {
    pub fn new(selected: bool, rule_type_text: &str, usage_text: &str, description: &str) -> Self {
        glib::Object::builder()
            .property("selected", selected)
            .property("rule-type-text", rule_type_text)
            .property("usage-text", usage_text)
            .property("description", description)
            .build()
    }
}
