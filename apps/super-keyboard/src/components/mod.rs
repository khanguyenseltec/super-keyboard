//! The app's components. Each reads what it needs from [`crate::state::AppState`]
//! in context, so components take no props beyond what they draw.

mod keyboard;
mod notices;
mod preview;
mod settings;
mod top_bar;

pub use keyboard::KeyboardView;
pub use notices::Notices;
pub use preview::Preview;
pub use settings::SettingsPanel;
pub use top_bar::TopBar;
