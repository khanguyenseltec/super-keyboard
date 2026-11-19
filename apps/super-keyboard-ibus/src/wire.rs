//! IBus objects as D-Bus values.
//!
//! IBus sends its objects ("serializables") as variants holding a struct: the
//! type name, a dictionary of attachments, then the object's fields in a fixed
//! order. New fields are only ever added at the end, so older and newer IBus
//! versions read each other's objects. The orders below follow libibus 1.5
//! (`ibustext.c`, `ibusproperty.c`, `ibusenginedesc.c`, `ibuscomponent.c`).

use std::collections::HashMap;

use zbus::zvariant::{OwnedValue, StructureBuilder, Value};

/// A serializable's struct, started with its type name and no attachments.
fn serializable(type_name: &'static str) -> StructureBuilder<'static> {
    let attachments: HashMap<String, OwnedValue> = HashMap::new();
    StructureBuilder::new()
        .add_field(type_name)
        .add_field(attachments)
}

/// Wraps a value as a variant field (`v`).
fn variant(value: Value<'static>) -> Value<'static> {
    Value::Value(Box::new(value))
}

fn build(builder: StructureBuilder<'static>) -> Value<'static> {
    Value::Structure(builder.build().expect("IBus structs always have fields"))
}

/// `IBusText`: plain text, or text underlined from start to end (pre-edit).
pub fn text(text: &str, underline: bool) -> Value<'static> {
    let attributes: Vec<Value<'static>> = if underline && !text.is_empty() {
        // IBUS_ATTR_TYPE_UNDERLINE (1), IBUS_ATTR_UNDERLINE_SINGLE (1), in
        // characters.
        let end = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
        vec![variant(build(
            serializable("IBusAttribute")
                .add_field(1u32)
                .add_field(1u32)
                .add_field(0u32)
                .add_field(end),
        ))]
    } else {
        Vec::new()
    };
    let attr_list = build(serializable("IBusAttrList").add_field(attributes));
    build(
        serializable("IBusText")
            .add_field(text.to_owned())
            .append_field(variant(attr_list)),
    )
}

/// The kinds of `IBusProperty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Normal = 0,
    Toggle = 1,
    Radio = 2,
    Separator = 4,
}

/// An `IBusProperty`: an entry in the input source menu of the desktop.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub key: String,
    pub kind: PropKind,
    pub label: String,
    pub tooltip: String,
    pub checked: bool,
    /// The short text the indicator shows, for the `InputMode` property.
    pub symbol: String,
    pub children: Vec<Prop>,
}

impl Prop {
    pub fn new(key: &str, kind: PropKind, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            kind,
            label: label.to_owned(),
            tooltip: String::new(),
            checked: false,
            symbol: String::new(),
            children: Vec::new(),
        }
    }

    pub fn to_value(&self) -> Value<'static> {
        build(
            serializable("IBusProperty")
                .add_field(self.key.clone())
                .add_field(self.kind as u32)
                .append_field(variant(text(&self.label, false)))
                .add_field(String::new()) // icon
                .append_field(variant(text(&self.tooltip, false)))
                .add_field(true) // sensitive
                .add_field(true) // visible
                .add_field(u32::from(self.checked))
                .append_field(variant(prop_list(&self.children)))
                .append_field(variant(text(&self.symbol, false))),
        )
    }
}

/// An `IBusPropList`.
pub fn prop_list(props: &[Prop]) -> Value<'static> {
    let props: Vec<Value<'static>> = props.iter().map(|p| variant(p.to_value())).collect();
    build(serializable("IBusPropList").add_field(props))
}

/// What the desktop lists for the engine (`IBusEngineDesc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDesc {
    pub name: &'static str,
    pub longname: &'static str,
    pub description: &'static str,
    pub language: &'static str,
    pub license: &'static str,
    pub author: &'static str,
    pub icon: String,
    pub layout: &'static str,
    pub symbol: &'static str,
    /// A command that opens the engine's settings.
    pub setup: String,
    pub version: &'static str,
}

impl EngineDesc {
    pub fn to_value(&self) -> Value<'static> {
        build(
            serializable("IBusEngineDesc")
                .add_field(self.name)
                .add_field(self.longname)
                .add_field(self.description)
                .add_field(self.language)
                .add_field(self.license)
                .add_field(self.author)
                .add_field(self.icon.clone())
                .add_field(self.layout)
                .add_field(0u32) // rank
                .add_field("") // hotkeys
                .add_field(self.symbol)
                .add_field(self.setup.clone())
                .add_field("") // layout variant
                .add_field("") // layout option
                .add_field(self.version)
                .add_field("") // text domain
                .add_field(""), // icon property key
        )
    }
}

/// An `IBusComponent`: the program that provides engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub license: &'static str,
    pub author: &'static str,
    pub homepage: &'static str,
    pub exec: String,
    pub engines: Vec<EngineDesc>,
}

impl Component {
    pub fn to_value(&self) -> Value<'static> {
        let observed_paths: Vec<Value<'static>> = Vec::new();
        let engines: Vec<Value<'static>> =
            self.engines.iter().map(|e| variant(e.to_value())).collect();
        build(
            serializable("IBusComponent")
                .add_field(self.name)
                .add_field(self.description)
                .add_field(self.version)
                .add_field(self.license)
                .add_field(self.author)
                .add_field(self.homepage)
                .add_field(self.exec.clone())
                .add_field("") // text domain
                .add_field(observed_paths)
                .add_field(engines),
        )
    }

    /// The component file IBus reads from `/usr/share/ibus/component/`.
    pub fn to_xml(&self) -> String {
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let mut xml = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<component>\n  <name>{}</name>\n  \
             <description>{}</description>\n  <exec>{}</exec>\n  <version>{}</version>\n  \
             <author>{}</author>\n  <license>{}</license>\n  <homepage>{}</homepage>\n  \
             <textdomain></textdomain>\n  <engines>\n",
            esc(self.name),
            esc(self.description),
            esc(&self.exec),
            esc(self.version),
            esc(self.author),
            esc(self.license),
            esc(self.homepage),
        );
        for e in &self.engines {
            xml.push_str(&format!(
                "    <engine>\n      <name>{}</name>\n      <longname>{}</longname>\n      \
                 <description>{}</description>\n      <language>{}</language>\n      \
                 <license>{}</license>\n      <author>{}</author>\n      <icon>{}</icon>\n      \
                 <layout>{}</layout>\n      <symbol>{}</symbol>\n      <setup>{}</setup>\n      \
                 <rank>0</rank>\n    </engine>\n",
                esc(e.name),
                esc(e.longname),
                esc(e.description),
                esc(e.language),
                esc(e.license),
                esc(e.author),
                esc(&e.icon),
                esc(e.layout),
                esc(e.symbol),
                esc(&e.setup),
            ));
        }
        xml.push_str("  </engines>\n</component>\n");
        xml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_has_the_ibus_shape() {
        let value = text("Việt", true);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}sv)");
        let Value::Structure(s) = &value else {
            panic!("{value:?}")
        };
        let fields = s.fields();
        assert_eq!(fields[0], Value::from("IBusText"));
        assert_eq!(fields[2], Value::from("Việt"));
        let Value::Value(attrs) = &fields[3] else {
            panic!("{fields:?}")
        };
        assert_eq!(attrs.value_signature().to_string(), "(sa{sv}av)");
        // The underline ends after 4 characters, not 6 bytes.
        assert!(format!("{attrs:?}").contains("U32(4)"), "{attrs:?}");
        let plain = text("x", false);
        assert!(!format!("{plain:?}").contains("IBusAttribute"));
    }

    #[test]
    fn properties_engines_and_components_have_the_ibus_shape() {
        let mut prop = Prop::new("InputMode", PropKind::Normal, "Tiếng Việt");
        prop.children
            .push(Prop::new("Method.telex", PropKind::Radio, "Telex"));
        assert_eq!(
            prop.to_value().value_signature().to_string(),
            "(sa{sv}suvsvbbuvv)"
        );
        let desc = EngineDesc {
            name: "e",
            longname: "E",
            description: "",
            language: "vi",
            license: "",
            author: "",
            icon: String::new(),
            layout: "us",
            symbol: "VI",
            setup: String::new(),
            version: "1",
        };
        assert_eq!(
            desc.to_value().value_signature().to_string(),
            "(sa{sv}ssssssssussssssss)"
        );
        let component = Component {
            name: "org.x",
            description: "",
            version: "1",
            license: "",
            author: "",
            homepage: "",
            exec: "/bin/x --ibus".into(),
            engines: vec![desc],
        };
        assert_eq!(
            component.to_value().value_signature().to_string(),
            "(sa{sv}ssssssssavav)"
        );
        let xml = component.to_xml();
        assert!(xml.contains("<exec>/bin/x --ibus</exec>"), "{xml}");
        assert!(xml.contains("<symbol>VI</symbol>"), "{xml}");
    }
}
