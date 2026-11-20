//! The IBus engine on D-Bus: the factory IBus asks for engines, the engine
//! that receives keys, and the menu it adds to the desktop's input indicator.

use std::sync::{Arc, Mutex};

use super_keyboard_core::{InputMethod, ToneStyle, VietnameseOptions};
use zbus::{
    Connection, ObjectServer, fdo, interface,
    message::Header,
    object_server::SignalEmitter,
    zvariant::{OwnedObjectPath, Value},
};

use crate::{
    keys::KeyHandler,
    sync::Shared,
    wire::{self, Prop, PropKind},
};

/// The engine's name in IBus and in the desktop's input sources.
pub const ENGINE_NAME: &str = "super-keyboard";
/// The bus name the engine process owns.
pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.SuperKeyboard";
pub const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// `IBUS_ENGINE_PREEDIT_COMMIT`: if focus leaves mid-syllable, IBus types
/// the syllable rather than dropping it.
const PREEDIT_COMMIT: u32 = 1;

/// Engines that are alive, for the settings watcher to update.
pub type Engines = Arc<Mutex<Vec<OwnedObjectPath>>>;

/// Creates an engine for each input context IBus switches to it.
#[derive(Debug)]
pub struct Factory {
    pub shared: Shared,
    pub engines: Engines,
    pub next_id: u32,
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    async fn create_engine(
        &mut self,
        name: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<OwnedObjectPath> {
        if name != ENGINE_NAME {
            return Err(fdo::Error::InvalidArgs(format!("no engine `{name}`")));
        }
        self.next_id += 1;
        let path = OwnedObjectPath::try_from(format!(
            "/org/freedesktop/IBus/Engine/SuperKeyboard/{}",
            self.next_id
        ))
        .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        server.at(&path, Engine::new(self.shared.clone())).await?;
        server
            .at(
                &path,
                Service {
                    engines: self.engines.clone(),
                },
            )
            .await?;
        lock(&self.engines).push(path.clone());
        Ok(path)
    }
}

/// `org.freedesktop.IBus.Service`: IBus destroys engines it no longer needs.
#[derive(Debug)]
struct Service {
    engines: Engines,
}

#[interface(name = "org.freedesktop.IBus.Service")]
impl Service {
    async fn destroy(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) {
        let Some(path) = header.path().map(|p| OwnedObjectPath::from(p.to_owned())) else {
            return;
        };
        lock(&self.engines).retain(|p| *p != path);
        // Removed after this call returns: an object cannot remove itself
        // while it is answering.
        let connection = connection.clone();
        tokio::spawn(async move {
            let server = connection.object_server();
            let _ = server.remove::<Engine, _>(&path).await;
            let _ = server.remove::<Service, _>(&path).await;
        });
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One engine instance.
#[derive(Debug)]
pub struct Engine {
    shared: Shared,
    handler: KeyHandler,
    /// The pre-edit text IBus is showing.
    shown: String,
}

impl Engine {
    fn new(shared: Shared) -> Self {
        let handler = KeyHandler::new(shared.options());
        Self {
            shared,
            handler,
            shown: String::new(),
        }
    }

    /// Picks up the shared settings, and commits the syllable in progress if
    /// they changed. Returns whether they changed.
    pub async fn refresh(&mut self, emitter: &SignalEmitter<'_>) -> zbus::Result<bool> {
        let options = self.shared.options();
        if options == self.handler.options() {
            return Ok(false);
        }
        let pending = self.handler.set_options(options);
        self.flush(emitter, pending).await?;
        Ok(true)
    }

    /// Shows the current pre-edit text, then commits `commit`.
    async fn flush(
        &mut self,
        emitter: &SignalEmitter<'_>,
        commit: Option<String>,
    ) -> zbus::Result<()> {
        let preedit = self.handler.preedit();
        if preedit != self.shown {
            let cursor = u32::try_from(preedit.chars().count()).unwrap_or(0);
            Self::update_preedit_text(
                emitter,
                wire::text(&preedit, true),
                cursor,
                !preedit.is_empty(),
                PREEDIT_COMMIT,
            )
            .await?;
            self.shown = preedit;
        }
        if let Some(text) = commit {
            Self::commit_text(emitter, wire::text(&text, false)).await?;
        }
        Ok(())
    }

    /// Sends the menu to the desktop.
    pub async fn show_menu(&self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        Self::register_properties(emitter, wire::prop_list(&menu(self.handler.options()))).await
    }

    /// Forgets the syllable: IBus commits the pre-edit text itself on focus
    /// changes and resets.
    fn forget(&mut self) {
        self.handler.reset();
        self.shown.clear();
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl Engine {
    async fn process_key_event(
        &mut self,
        keyval: u32,
        _keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let _ = self.refresh(&emitter).await;
        let response = self.handler.key(keyval, state);
        let _ = self.flush(&emitter, response.commit).await;
        response.handled
    }

    async fn property_activate(
        &mut self,
        name: String,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let checked = state == 1;
        let changed = match name.as_str() {
            "InputMode" => self.shared.update(|o| o.enabled = !o.enabled),
            key if key.starts_with("Method.") => match method_of(key) {
                Some(method) if checked => self.shared.update(|o| o.method = method),
                _ => return,
            },
            "Tone.classic" if checked => self.shared.update(|o| o.tone_style = ToneStyle::Classic),
            "Tone.modern" if checked => self.shared.update(|o| o.tone_style = ToneStyle::Modern),
            "CheckSpelling" => self.shared.update(|o| o.check_spelling = checked),
            "Setup" => {
                open_app();
                return;
            }
            _ => return,
        };
        if let Err(err) = changed {
            eprintln!("super-keyboard-ibus: settings not saved: {err}");
        }
        let _ = self.refresh(&emitter).await;
        let _ = self.show_menu(&emitter).await;
    }

    async fn focus_in(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        let _ = self.refresh(&emitter).await;
        let _ = self.show_menu(&emitter).await;
    }

    async fn focus_in_id(
        &mut self,
        _object_path: String,
        _client: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.focus_in(emitter).await;
    }

    async fn enable(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.focus_in(emitter).await;
    }

    fn focus_out(&mut self) {
        self.forget();
    }

    fn focus_out_id(&mut self, _object_path: String) {
        self.forget();
    }

    fn reset(&mut self) {
        self.forget();
    }

    fn disable(&mut self) {
        self.forget();
    }

    // The rest of the interface: nothing to do for a keyboard engine.
    fn set_capabilities(&self, _caps: u32) {}
    fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    fn set_surrounding_text(&self, _text: Value<'_>, _cursor: u32, _anchor: u32) {}
    fn property_show(&self, _name: String) {}
    fn property_hide(&self, _name: String) {}
    fn candidate_clicked(&self, _index: u32, _button: u32, _state: u32) {}
    fn page_up(&self) {}
    fn page_down(&self) {}
    fn cursor_up(&self) {}
    fn cursor_down(&self) {}
    fn process_hand_writing_event(&self, _coordinates: Vec<f64>) {}
    fn cancel_hand_writing(&self, _n_strokes: u32) {}
    fn panel_extension_received(&self, _event: Value<'_>) {}
    fn panel_extension_register_keys(&self, _data: Value<'_>) {}

    /// IBus 1.5.27+ asks whether to call `FocusInId` instead of `FocusIn`.
    #[zbus(property(emits_changed_signal = "const"), name = "FocusId")]
    fn focus_id(&self) -> bool {
        false
    }

    #[zbus(
        property(emits_changed_signal = "const"),
        name = "ActiveSurroundingText"
    )]
    fn active_surrounding_text(&self) -> bool {
        false
    }

    #[zbus(signal, name = "CommitText")]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "UpdatePreeditText")]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal, name = "RegisterProperties")]
    async fn register_properties(emitter: &SignalEmitter<'_>, props: Value<'_>)
    -> zbus::Result<()>;
}

/// The engine's entries in the desktop's input indicator. `InputMode`'s
/// symbol (VI or EN) is what GNOME shows in the top bar.
fn menu(options: VietnameseOptions) -> Vec<Prop> {
    let on = options.enabled;
    let mut mode = Prop::new(
        "InputMode",
        PropKind::Normal,
        if on {
            "Tiếng Việt is on: switch to English"
        } else {
            "English: switch to Tiếng Việt"
        },
    );
    mode.symbol = if on { "VI" } else { "EN" }.to_owned();
    mode.tooltip = "Shared with the Super Keyboard app".to_owned();
    let radio = |key: &str, label: &str, checked: bool| Prop {
        checked,
        ..Prop::new(key, PropKind::Radio, label)
    };
    let separator = |key: &str| Prop::new(key, PropKind::Separator, "");
    let mut items = vec![mode, separator("Separator.method")];
    items.extend(InputMethod::ALL.map(|method| {
        radio(
            &format!("Method.{}", method.name()),
            method.label(),
            options.method == method,
        )
    }));
    items.extend([
        separator("Separator.tone"),
        radio(
            "Tone.classic",
            "Classic tones: hòa, thủy",
            options.tone_style == ToneStyle::Classic,
        ),
        radio(
            "Tone.modern",
            "Modern tones: hoà, thuỷ",
            options.tone_style == ToneStyle::Modern,
        ),
        separator("Separator.options"),
        Prop {
            checked: options.check_spelling,
            ..Prop::new("CheckSpelling", PropKind::Toggle, "Check spelling")
        },
        Prop::new("Setup", PropKind::Normal, "Open Super Keyboard…"),
    ]);
    items
}

/// The method a `Method.<name>` menu entry stands for.
fn method_of(key: &str) -> Option<InputMethod> {
    key.strip_prefix("Method.").and_then(InputMethod::from_name)
}

/// Opens the Super Keyboard app, for its Tiếng Việt settings.
fn open_app() {
    let installed = ["/usr/bin/super-keyboard", "super-keyboard"];
    for program in installed {
        if std::process::Command::new(program).spawn().is_ok() {
            return;
        }
    }
    eprintln!("super-keyboard-ibus: the Super Keyboard app is not installed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_shows_the_settings() {
        let options = VietnameseOptions {
            enabled: true,
            method: InputMethod::Vni,
            ..VietnameseOptions::default()
        };
        let items = menu(options);
        assert_eq!(items[0].symbol, "VI");
        let checked: Vec<&str> = items
            .iter()
            .filter(|p| p.checked)
            .map(|p| p.key.as_str())
            .collect();
        assert_eq!(checked, ["Method.vni", "Tone.classic", "CheckSpelling"]);
        assert_eq!(menu(VietnameseOptions::default())[0].symbol, "EN");
    }

    #[test]
    fn the_menu_offers_every_method() {
        for method in InputMethod::ALL {
            let items = menu(VietnameseOptions {
                method,
                ..VietnameseOptions::default()
            });
            let key = format!("Method.{}", method.name());
            let item = items.iter().find(|p| p.key == key);
            assert!(item.is_some_and(|p| p.checked), "{key}");
            assert_eq!(method_of(&key), Some(method));
        }
        assert_eq!(method_of("Method.unikey"), None);
        assert_eq!(method_of("Tone.classic"), None);
    }
}
