//! `super-keyboard-ibus`: Vietnamese typing with the physical keyboard, in
//! every application, as an IBus input method engine.
//!
//! It uses the same Telex, VNI and VIQR rules as the on-screen keyboard and shares
//! its `[vietnamese]` settings through `settings.toml`, so switching VI / EN or
//! the method on either side changes both.
//!
//! ```text
//! super-keyboard-ibus --ibus        started by IBus (the installed component)
//! super-keyboard-ibus --register    run now and register with the running IBus,
//!                                   no install needed: then `ibus engine super-keyboard`
//! super-keyboard-ibus --xml [EXEC]  print the component file for /usr/share/ibus/component
//! ```

mod engine;
mod keys;
mod sync;
mod wire;

use std::{env, fs, path::PathBuf, process::ExitCode, time::Duration};

use engine::{COMPONENT_NAME, ENGINE_NAME, Engine, Engines, FACTORY_PATH, Factory};
use sync::Shared;
use wire::{Component, EngineDesc};
use zbus::Connection;

/// How the installed engine is started (`/usr/libexec`, as Debian puts IBus
/// engines).
#[cfg(test)]
const INSTALLED_EXEC: &str = "/usr/libexec/super-keyboard-ibus --ibus";

/// How often `settings.toml` is checked for changes made by the app.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--ibus") => serve(false),
        Some("--register") | None => serve(true),
        Some("--xml") => {
            let exec = args
                .get(1)
                .cloned()
                .unwrap_or_else(|| format!("{} --ibus", current_exe()));
            print!("{}", component(exec).to_xml());
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") => {
            println!(
                "super-keyboard-ibus: Vietnamese typing (Telex, VNI, VIQR) for IBus, shared with Super Keyboard\n\n\
                 --ibus              run as started by IBus\n\
                 --register          register with the running IBus now (no install needed)\n\
                 --xml [EXEC]        print the IBus component file"
            );
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("super-keyboard-ibus: unknown option `{other}` (try --help)");
            ExitCode::FAILURE
        }
    }
}

fn current_exe() -> String {
    env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/usr/libexec/super-keyboard-ibus".to_owned())
}

/// What IBus lists: one engine, Vietnamese, on top of the US layout.
fn component(exec: String) -> Component {
    Component {
        name: COMPONENT_NAME,
        description: "Super Keyboard: Vietnamese typing with Telex, VNI or VIQR",
        version: env!("CARGO_PKG_VERSION"),
        license: "MIT OR Apache-2.0",
        author: "Super Keyboard contributors",
        homepage: "https://github.com/tristanseltec/super-keyboard",
        exec,
        engines: vec![EngineDesc {
            name: ENGINE_NAME,
            longname: "Vietnamese (Super Keyboard)",
            description: "Telex, VNI and VIQR, shared with the Super Keyboard on-screen keyboard",
            language: "vi",
            license: "MIT OR Apache-2.0",
            author: "Super Keyboard contributors",
            // Installed by the .deb into the hicolor icon theme.
            icon: "super-keyboard".to_owned(),
            layout: "us",
            symbol: "VI",
            setup: "/usr/bin/super-keyboard".to_owned(),
            version: env!("CARGO_PKG_VERSION"),
        }],
    }
}

fn serve(register: bool) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("super-keyboard-ibus: {err}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run(register)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("super-keyboard-ibus: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(register: bool) -> Result<(), String> {
    let address = ibus_address()?;
    let shared = Shared::load();
    let engines = Engines::default();
    let factory = Factory {
        shared: shared.clone(),
        engines: engines.clone(),
        next_id: 0,
    };
    let connection = zbus::connection::Builder::address(address.as_str())
        .and_then(|b| b.name(COMPONENT_NAME))
        .and_then(|b| b.serve_at(FACTORY_PATH, factory))
        .map_err(|err| err.to_string())?
        .build()
        .await
        .map_err(|err| format!("cannot join IBus at {address}: {err}"))?;

    if register {
        connection
            .call_method(
                Some("org.freedesktop.IBus"),
                "/org/freedesktop/IBus",
                Some("org.freedesktop.IBus"),
                "RegisterComponent",
                &(component(format!("{} --ibus", current_exe())).to_value(),),
            )
            .await
            .map_err(|err| format!("IBus refused the component: {err}"))?;
        eprintln!(
            "super-keyboard-ibus: registered. Switch to it with `ibus engine {ENGINE_NAME}`; \
             Ctrl+C stops it."
        );
    }

    watch(&connection, &shared, &engines).await
}

/// Follows `settings.toml` and tells live engines when it changes; returns when
/// IBus goes away.
async fn watch(connection: &Connection, shared: &Shared, engines: &Engines) -> Result<(), String> {
    let dbus = zbus::fdo::DBusProxy::new(connection)
        .await
        .map_err(|err| err.to_string())?;
    let mut ticks = 0u32;
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        if shared.poll() {
            let paths = engines.lock().map(|e| e.clone()).unwrap_or_default();
            for path in paths {
                let Ok(engine) = connection
                    .object_server()
                    .interface::<_, Engine>(&path)
                    .await
                else {
                    continue;
                };
                let emitter = engine.signal_emitter().clone();
                let mut engine = engine.get_mut().await;
                if engine.refresh(&emitter).await.unwrap_or(false) {
                    let _ = engine.show_menu(&emitter).await;
                }
            }
        }
        // Every ten seconds, check that IBus is still there.
        ticks += 1;
        if ticks.is_multiple_of(10) {
            let ibus = "org.freedesktop.IBus".try_into().map_err(|_| "bad name")?;
            if !dbus.name_has_owner(ibus).await.unwrap_or(false) {
                return Ok(());
            }
        }
    }
}

/// Where IBus's private bus is: `$IBUS_ADDRESS` (set when IBus starts the
/// engine), or the newest address file IBus wrote for this user.
fn ibus_address() -> Result<String, String> {
    if let Ok(address) = env::var("IBUS_ADDRESS")
        && !address.is_empty()
    {
        return Ok(address);
    }
    let config = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or("no home directory")?;
    let dir = config.join("ibus").join("bus");
    let newest = fs::read_dir(&dir)
        .map_err(|err| format!("IBus is not running ({}: {err})", dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .max()
        .map(|(_, path)| path)
        .ok_or_else(|| format!("IBus is not running (nothing in {})", dir.display()))?;
    fs::read_to_string(&newest)
        .map_err(|err| err.to_string())?
        .lines()
        .find_map(|line| line.strip_prefix("IBUS_ADDRESS="))
        .map(str::to_owned)
        .ok_or_else(|| format!("no IBUS_ADDRESS in {}", newest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packaged_component_file_is_up_to_date() {
        let generated = component(INSTALLED_EXEC.into()).to_xml();
        let packaged = include_str!("../data/super-keyboard.xml");
        assert_eq!(
            packaged, generated,
            "regenerate with: super-keyboard-ibus --xml \"{INSTALLED_EXEC}\" > apps/super-keyboard-ibus/data/super-keyboard.xml"
        );
    }

    #[test]
    fn the_component_file_names_the_engine() {
        let xml = component("/usr/libexec/super-keyboard-ibus --ibus".into()).to_xml();
        for part in [
            "<name>org.freedesktop.IBus.SuperKeyboard</name>",
            "<exec>/usr/libexec/super-keyboard-ibus --ibus</exec>",
            "<name>super-keyboard</name>",
            "<language>vi</language>",
            "<layout>us</layout>",
        ] {
            assert!(xml.contains(part), "{part}\n{xml}");
        }
    }
}
