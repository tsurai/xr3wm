use layout::*;
use keycode::*;
use std::default::Default;
use workspaces::WorkspaceConfig;
use commands::{Cmd, CmdManage};

include!(concat!(env!("HOME"), "/.xr3wm/config.rs"))

pub struct Keybinding {
  pub mods: u8,
  pub key: String,
  pub cmd: Cmd
}

pub struct ManageHook {
  pub class_name: String,
  pub cmd: CmdManage
}

pub struct Config {
  pub workspaces: Vec<WorkspaceConfig>,
  pub mod_key: u8,
  pub border_width: u32,
  pub border_color: u32,
  pub border_focus_color: u32,
  pub keybindings: Vec<Keybinding>,
  pub manage_hooks: Vec<ManageHook>
}

impl Default for Config {
  fn default() -> Config {
    let mut config = Config {
      workspaces: Vec::from_fn(9, |idx| WorkspaceConfig{tag: (idx + 1).to_string(), screen: 0, layout: || { box TallLayout::new(1, 0.5, 0.01) }}),
      mod_key: MOD_4,
      border_width: 2,
      border_color: 0x002e2e2e,
      border_focus_color: 0x002a82e6,
      keybindings: vec![
        Keybinding {
          mods: 0,
          key: String::from_str("Return"),
          cmd: Cmd::Exec(String::from_str("xterm -u8"))
        },
        Keybinding {
          mods: 0,
          key: String::from_str("d"),
          cmd: Cmd::Exec(String::from_str("dmenu_run"))
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: String::from_str("q"),
          cmd: Cmd::KillClient
        },
        Keybinding {
          mods: 0,
          key: String::from_str("j"),
          cmd: Cmd::FocusDown
        },
        Keybinding {
          mods: 0,
          key: String::from_str("k"),
          cmd: Cmd::FocusUp
        },
        Keybinding {
          mods: 0,
          key: String::from_str("m"),
          cmd: Cmd::FocusMaster
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: String::from_str("j"),
          cmd: Cmd::SwapDown
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: String::from_str("k"),
          cmd: Cmd::SwapUp
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: String::from_str("Return"),
          cmd: Cmd::SwapMaster
        },
      ],
      manage_hooks: Vec::new()
    };

    for i in range(1u, 10) {
      config.keybindings.push(
        Keybinding {
          mods: 0,
          key: i.to_string(),
          cmd: Cmd::SwitchWorkspace(i)
        });

      config.keybindings.push(
        Keybinding {
          mods: MOD_SHIFT,
          key: i.to_string(),
          cmd: Cmd::MoveToWorkspace(i)
        });
    }

    for &(i, key) in vec![(1, "w"), (2, "e"), (3, "r")].iter() {
      config.keybindings.push(
        Keybinding {
          mods: 0,
          key: String::from_str(key),
          cmd: Cmd::SwitchScreen(i)
        });

      config.keybindings.push(
        Keybinding {
          mods: MOD_SHIFT,
          key: String::from_str(key),
          cmd: Cmd::MoveToScreen(i)
        });
    }

    config
  }
}
