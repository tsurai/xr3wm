use layout;
use keycode::{MOD_4, Keystroke};
use std::rc::Rc;
use std::default::Default;
use workspaces::WorkspaceConfig;

include!(concat!(env!("HOME"), "/.xr3wm/config.rs"))

pub struct Config {
  pub workspaces: Vec<WorkspaceConfig>,
  pub mod_key: u8,
  pub border_width: u32,
  pub border_color: u64,
  pub border_focus_color: u64,
  pub terminal: String,
  pub terminal_shortcut: Keystroke,
  pub launcher: String,
  pub launcher_shortcut: Keystroke
}

impl Default for Config {
  fn default() -> Config {
    Config {
      workspaces: Vec::from_fn(9, |idx| WorkspaceConfig{tag: (idx + 1).to_string(), screen: 0, layout: layout::to_box(layout::TallLayout::new(1, 0.5, 0.01))}),
      mod_key: MOD_4,
      border_width: 1,
      border_color: 0x000000ff,
      border_focus_color: 0x00ff0000,
      terminal: String::from_str("xterm"),
      terminal_shortcut: Keystroke{mods: 0, key: String::from_str("Return")},
      launcher: String::from_str("dmenu_run"),
      launcher_shortcut: Keystroke{mods: 0, key: String::from_str("d")},
    }
  }
}