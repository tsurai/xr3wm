use layout;
use keycode::*;
use std::rc::Rc;
use std::default::Default;
use workspaces::WorkspaceConfig;

include!(concat!(env!("HOME"), "/.xr3wm/config.rs"))

pub struct Config {
  pub workspaces: Vec<WorkspaceConfig>,
  pub mod_key: u8,
  pub border_width: u32,
  pub border_color: u32,
  pub border_focus_color: u32,
  pub terminal: String,
  pub terminal_shortcut: Keystroke,
  pub launcher: String,
  pub launcher_shortcut: Keystroke,
  pub kill_shortcut: Keystroke
}

impl Default for Config {
  fn default() -> Config {
    Config {
      workspaces: Vec::from_fn(9, |idx| WorkspaceConfig{tag: (idx + 1).to_string(), screen: 0, layout: layout::to_box(layout::TallLayout::new(1, 0.5, 0.01))}),
      mod_key: MOD_4,
      border_width: 2,
      border_color: 0x002e2e2e,
      border_focus_color: 0x002a82e6,
      terminal: String::from_str("xterm"),
      terminal_shortcut: Keystroke{mods: 0, key: String::from_str("Return")},
      launcher: String::from_str("dmenu_run"),
      launcher_shortcut: Keystroke{mods: 0, key: String::from_str("d")},
      kill_shortcut: Keystroke{mods: MOD_SHIFT, key: String::from_str("q")}
    }
  }
}
