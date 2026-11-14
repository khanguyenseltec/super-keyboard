//! Checks a parsed layout file and turns it into a [`Layout`].

use std::collections::HashSet;

use super::{
    Key, LayerDef, Layout, Section, Swipe,
    error::{Block, KeyProblem, LayoutError},
    schema::{KeyFile, KeyTable, LayerFile, LayoutFile, SectionFile},
};
use crate::action::Action;

pub(super) fn resolve(file: LayoutFile) -> Result<Layout, LayoutError> {
    if file.layers.is_empty() {
        return Err(LayoutError::NoLayers);
    }
    let mut layer_names = HashSet::with_capacity(file.layers.len());
    for layer in &file.layers {
        if !layer_names.insert(layer.name.as_str()) {
            return Err(LayoutError::DuplicateLayer(layer.name.clone()));
        }
    }

    let mut sections = Vec::with_capacity(file.sections.len() + file.layers.len());
    for section in &file.sections {
        if sections.iter().any(|s: &Section| s.name == section.name) {
            return Err(LayoutError::DuplicateSection(section.name.clone()));
        }
        sections.push(resolve_section(section, &layer_names)?);
    }
    let declared = sections.len();

    let mut layers = Vec::with_capacity(file.layers.len());
    for layer in &file.layers {
        let indices = match (&layer.sections, &layer.rows) {
            (Some(names), None) => section_indices(layer, names, &sections[..declared])?,
            (None, Some(rows)) => {
                // A layer's own rows become a required section named after it.
                if sections.iter().any(|s| s.name == layer.name) {
                    return Err(LayoutError::DuplicateSection(layer.name.clone()));
                }
                let block = Block::Layer(layer.name.clone());
                let rows = resolve_rows(&block, rows, &layer_names)?;
                if rows.iter().all(Vec::is_empty) {
                    return Err(LayoutError::EmptyLayer(layer.name.clone()));
                }
                sections.push(Section {
                    name: layer.name.clone(),
                    title: layer.name.clone(),
                    rows,
                    enabled: true,
                    required: true,
                });
                vec![sections.len() - 1]
            }
            _ => return Err(LayoutError::LayerContent(layer.name.clone())),
        };
        layers.push(LayerDef {
            name: layer.name.clone(),
            sections: indices,
        });
    }

    Ok(Layout {
        name: file.name,
        description: file.description,
        sections,
        layers,
    })
}

fn resolve_section(section: &SectionFile, layers: &HashSet<&str>) -> Result<Section, LayoutError> {
    let name = &section.name;
    let is_valid_name = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if !is_valid_name {
        return Err(LayoutError::InvalidSectionName(name.clone()));
    }
    let rows = resolve_rows(&Block::Section(name.clone()), &section.rows, layers)?;
    if rows.iter().all(Vec::is_empty) {
        return Err(LayoutError::EmptySection(name.clone()));
    }
    let required = section.required.unwrap_or(false);
    let enabled = section.enabled.unwrap_or(true);
    if required && !enabled {
        return Err(LayoutError::RequiredSectionOff(name.clone()));
    }
    Ok(Section {
        name: name.clone(),
        title: section.title.clone().unwrap_or_else(|| name.clone()),
        rows,
        enabled,
        required,
    })
}

/// The indices of the sections a layer lists, which must follow declaration
/// order so that one order can apply to every layer.
fn section_indices(
    layer: &LayerFile,
    names: &[String],
    declared: &[Section],
) -> Result<Vec<usize>, LayoutError> {
    if names.is_empty() {
        return Err(LayoutError::EmptyLayer(layer.name.clone()));
    }
    let mut indices: Vec<usize> = Vec::with_capacity(names.len());
    for name in names {
        let Some(index) = declared.iter().position(|s| s.name == *name) else {
            return Err(LayoutError::UnknownSection {
                layer: layer.name.clone(),
                section: name.clone(),
            });
        };
        if indices.contains(&index) {
            return Err(LayoutError::RepeatedSection {
                layer: layer.name.clone(),
                section: name.clone(),
            });
        }
        if let Some(&previous) = indices.last()
            && index < previous
        {
            return Err(LayoutError::SectionOrder {
                layer: layer.name.clone(),
                earlier: declared[previous].name.clone(),
                later: name.clone(),
            });
        }
        indices.push(index);
    }
    Ok(indices)
}

fn resolve_rows(
    block: &Block,
    rows: &[Vec<KeyFile>],
    layers: &HashSet<&str>,
) -> Result<Vec<Vec<Key>>, LayoutError> {
    rows.iter()
        .enumerate()
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .map(|(k, key)| {
                    resolve_key(key, layers).map_err(|problem| LayoutError::InvalidKey {
                        block: block.clone(),
                        row: r + 1,
                        key: k + 1,
                        problem,
                    })
                })
                .collect()
        })
        .collect()
}

fn resolve_key(key: &KeyFile, layers: &HashSet<&str>) -> Result<Key, KeyProblem> {
    let short;
    let table = match key {
        KeyFile::Short(spec) => {
            short = KeyTable {
                tap: Some(spec.clone()),
                ..KeyTable::default()
            };
            &short
        }
        KeyFile::Table(table) => table,
    };

    let tap = match (&table.tap, &table.text) {
        (Some(spec), None) => Action::parse(spec).map_err(KeyProblem::Tap)?,
        (None, Some(text)) if !text.is_empty() => Action::Text(text.clone()),
        (None, Some(_)) => return Err(KeyProblem::EmptyText),
        _ => return Err(KeyProblem::TapOrText),
    };
    let hold = table
        .hold
        .as_deref()
        .map(Action::parse)
        .transpose()
        .map_err(KeyProblem::Hold)?;
    for action in std::iter::once(&tap).chain(&hold) {
        check_layer_target(action, layers)?;
    }

    let swipe = table
        .swipe
        .as_deref()
        .map(|name| Swipe::from_name(name).ok_or_else(|| KeyProblem::UnknownSwipe(name.into())))
        .transpose()?;

    // Repeating, holding and swiping all start by keeping a finger down, so a key
    // can only do one of them. Explicit `repeat = true` conflicts; the default
    // repeat of Backspace and the arrows quietly gives way.
    let repeat = match table.repeat {
        Some(true) if hold.is_some() => return Err(KeyProblem::RepeatConflict("hold")),
        Some(true) if swipe.is_some() => return Err(KeyProblem::RepeatConflict("swipe")),
        Some(repeat) => repeat,
        None => tap.repeats_by_default() && hold.is_none() && swipe.is_none(),
    };

    let width = table.width.unwrap_or(1.0);
    if !(width.is_finite() && width > 0.0) {
        return Err(KeyProblem::InvalidWidth(width));
    }

    Ok(Key {
        label: table.label.clone().unwrap_or_else(|| tap.default_label()),
        tap,
        hold,
        swipe,
        repeat,
        width,
    })
}

fn check_layer_target(action: &Action, layers: &HashSet<&str>) -> Result<(), KeyProblem> {
    match action {
        Action::Layer(target) if !layers.contains(target.as_str()) => {
            Err(KeyProblem::UnknownLayer(target.clone()))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::action::ParseActionError;
    use crate::layout::{KeyProblem, Layout, LayoutError};

    /// A one-layer layout with `keys` as its only row.
    fn row(keys: &str) -> Result<Layout, LayoutError> {
        Layout::from_toml(&format!(
            "name = \"t\"\n[[layers]]\nname = \"a\"\nrows = [[{keys}]]"
        ))
    }

    fn problem(keys: &str) -> KeyProblem {
        match row(keys) {
            Err(LayoutError::InvalidKey { problem, .. }) => problem,
            other => panic!("expected a key problem for {keys}, got {other:?}"),
        }
    }

    #[test]
    fn rejects_bad_layouts() {
        let bad = [
            r#"name = "t""#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [["x"]]
               [[layers]]
               name = "a"
               rows = [["y"]]"#,
            r#"name = "t"
               [[layers]]
               name = "a"
               rows = [[]]"#,
        ];
        for source in bad {
            assert!(Layout::from_toml(source).is_err(), "accepted:\n{source}");
        }
    }

    #[test]
    fn names_each_key_problem() {
        assert_eq!(
            problem(r#""<nope>""#),
            KeyProblem::Tap(ParseActionError::UnknownName("nope".into()))
        );
        assert_eq!(
            problem(r#"{ tap = "x", hold = "<nope>" }"#),
            KeyProblem::Hold(ParseActionError::UnknownName("nope".into()))
        );
        assert_eq!(
            problem(r#""<layer:missing>""#),
            KeyProblem::UnknownLayer("missing".into())
        );
        assert_eq!(
            problem(r#"{ tap = "x", hold = "<layer:missing>" }"#),
            KeyProblem::UnknownLayer("missing".into())
        );
        assert_eq!(
            problem(r#"{ tap = "x", text = "y" }"#),
            KeyProblem::TapOrText
        );
        assert_eq!(problem(r#"{ label = "x" }"#), KeyProblem::TapOrText);
        assert_eq!(problem(r#"{ text = "" }"#), KeyProblem::EmptyText);
        assert_eq!(
            problem(r#"{ tap = "x", width = 0.0 }"#),
            KeyProblem::InvalidWidth(0.0)
        );
        assert_eq!(
            problem(r#"{ tap = "x", swipe = "zoom" }"#),
            KeyProblem::UnknownSwipe("zoom".into())
        );
        assert_eq!(
            problem(r#"{ tap = "<bksp>", hold = "x", repeat = true }"#),
            KeyProblem::RepeatConflict("hold")
        );
        assert_eq!(
            problem(r#"{ tap = "<space>", swipe = "cursor", repeat = true }"#),
            KeyProblem::RepeatConflict("swipe")
        );
    }

    #[test]
    fn errors_point_at_the_key() {
        let err = Layout::from_toml(
            r#"name = "t"
               [[layers]]
               name = "base"
               rows = [["a", "b"], ["c", "<nope>"]]"#,
        )
        .unwrap_err();
        assert!(
            err.to_string().starts_with("layer `base`, row 2, key 2:"),
            "{err}"
        );
    }

    #[test]
    fn duplicate_layers_are_named() {
        let err = Layout::from_toml(
            r#"name = "t"
               [[layers]]
               name = "x"
               rows = [["a"]]
               [[layers]]
               name = "x"
               rows = [["b"]]"#,
        )
        .unwrap_err();
        assert_eq!(err, LayoutError::DuplicateLayer("x".into()));
    }

    /// A layout with sections `a` and `b` and the given extra TOML.
    fn sections(extra: &str) -> Result<Layout, LayoutError> {
        Layout::from_toml(&format!(
            "name = \"t\"\n\
             [[sections]]\nname = \"a\"\nrows = [[\"x\"]]\n\
             [[sections]]\nname = \"b\"\nrows = [[\"y\"]]\n{extra}"
        ))
    }

    #[test]
    fn checks_sections_and_how_layers_use_them() {
        let layer = |list: &str| sections(&format!("[[layers]]\nname = \"l\"\nsections = {list}"));
        assert!(layer(r#"["a", "b"]"#).is_ok());
        assert!(layer(r#"["b"]"#).is_ok());
        assert_eq!(
            layer(r#"["b", "a"]"#).unwrap_err(),
            LayoutError::SectionOrder {
                layer: "l".into(),
                earlier: "b".into(),
                later: "a".into()
            }
        );
        assert_eq!(
            layer(r#"["a", "a"]"#).unwrap_err(),
            LayoutError::RepeatedSection {
                layer: "l".into(),
                section: "a".into()
            }
        );
        assert_eq!(
            layer(r#"["c"]"#).unwrap_err(),
            LayoutError::UnknownSection {
                layer: "l".into(),
                section: "c".into()
            }
        );
        assert_eq!(
            layer("[]").unwrap_err(),
            LayoutError::EmptyLayer("l".into())
        );
        assert_eq!(
            sections("[[layers]]\nname = \"l\"\nsections = [\"a\"]\nrows = [[\"z\"]]").unwrap_err(),
            LayoutError::LayerContent("l".into())
        );
        assert_eq!(
            sections("[[layers]]\nname = \"l\"").unwrap_err(),
            LayoutError::LayerContent("l".into())
        );
        // A layer's own rows become a section named after the layer.
        assert_eq!(
            sections("[[layers]]\nname = \"a\"\nrows = [[\"z\"]]").unwrap_err(),
            LayoutError::DuplicateSection("a".into())
        );
    }

    #[test]
    fn checks_each_section() {
        let section = |body: &str| {
            Layout::from_toml(&format!(
                "name = \"t\"\n[[sections]]\n{body}\n[[layers]]\nname = \"l\"\nsections = [\"s\"]"
            ))
        };
        assert!(section("name = \"s\"\nrows = [[\"x\"]]").is_ok());
        assert_eq!(
            section("name = \"s s\"\nrows = [[\"x\"]]").unwrap_err(),
            LayoutError::InvalidSectionName("s s".into())
        );
        assert_eq!(
            section("name = \"s\"\nrows = [[]]").unwrap_err(),
            LayoutError::EmptySection("s".into())
        );
        assert_eq!(
            section("name = \"s\"\nrequired = true\nenabled = false\nrows = [[\"x\"]]")
                .unwrap_err(),
            LayoutError::RequiredSectionOff("s".into())
        );
        let err = section("name = \"s\"\nrows = [[\"<nope>\"]]").unwrap_err();
        assert!(
            err.to_string().starts_with("section `s`, row 1, key 1:"),
            "{err}"
        );
        assert!(matches!(
            sections("[[sections]]\nname = \"a\"\nrows = [[\"z\"]]\n[[layers]]\nname = \"l\"\nsections = [\"a\"]"),
            Err(LayoutError::DuplicateSection(name)) if name == "a"
        ));
    }
}
