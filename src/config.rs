use keysym;
use layout;
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
  pub terminal: String
}

impl Default for Config {
  fn default() -> Config {
    Config{
      workspaces: Vec::from_fn(9, |idx| WorkspaceConfig{tag: (idx + 1).to_string(), screen: 0, layout: layout::to_box(layout::TallLayout::new(1))}),
      mod_key: keysym::MOD_4,
      border_width: 1,
      border_color: 0x00ff0000,
      border_focus_color: 0x0000ff00,
      terminal: String::from_str("xterm"),
    }
  }
}