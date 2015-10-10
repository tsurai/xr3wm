#![allow(dead_code)]

use std::default::Default;
use std::io::Write;
use std::path::Path;
use std::fs::{PathExt, File, create_dir};
use std::process::Command;
use std::mem;
use layout::*;
use keycode::*;
use workspaces::WorkspaceConfig;
use commands::{Cmd, ManageHook, LogHook};
use dylib::DynamicLibrary;

pub struct Keybinding {
  pub mods: u8,
  pub key: String,
  pub cmd: Cmd
}

pub struct Config {
  lib: Option<DynamicLibrary>,
  pub workspaces: Vec<WorkspaceConfig>,
  pub mod_key: u8,
  pub border_width: u32,
  pub border_color: u32,
  pub border_focus_color: u32,
  pub border_urgent_color: u32,
  pub greedy_view: bool,
  pub keybindings: Vec<Keybinding>,
  pub manage_hooks: Vec<ManageHook>,
  pub log_hook: Option<LogHook>
}

impl Default for Config {
  fn default() -> Config {
    let mut config = Config {
      lib: None,
      workspaces: (1usize..10).map(|idx| WorkspaceConfig { tag: idx.to_string(), screen: 0, layout: TallLayout::new(1, 0.5, 0.05) }).collect(),
      mod_key: MOD_4,
      border_width: 2,
      border_color: 0x002e2e2e,
      border_focus_color: 0x002a82e6,
      border_urgent_color: 0x00ff0000,
      greedy_view: false,
      keybindings: vec![
        Keybinding {
          mods: 0,
          key: "Return".to_string(),
          cmd: Cmd::Exec("xterm -u8".to_string())
        },
        Keybinding {
          mods: 0,
          key: "d".to_string(),
          cmd: Cmd::Exec("dmenu_run".to_string())
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "q".to_string(),
          cmd: Cmd::KillClient
        },
        Keybinding {
          mods: 0,
          key: "j".to_string(),
          cmd: Cmd::FocusDown
        },
        Keybinding {
          mods: 0,
          key: "k".to_string(),
          cmd: Cmd::FocusUp
        },
        Keybinding {
          mods: 0,
          key: "m".to_string(),
          cmd: Cmd::FocusMaster
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "j".to_string(),
          cmd: Cmd::SwapDown
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "k".to_string(),
          cmd: Cmd::SwapUp
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "Return".to_string(),
          cmd: Cmd::SwapMaster
        },
        Keybinding {
          mods: 0,
          key: "comma".to_string(),
          cmd: Cmd::SendLayoutMsg(LayoutMsg::IncreaseMaster)
        },
        Keybinding {
          mods: 0,
          key: "period".to_string(),
          cmd: Cmd::SendLayoutMsg(LayoutMsg::DecreaseMaster)
        },
        Keybinding {
          mods: 0,
          key: "l".to_string(),
          cmd: Cmd::SendLayoutMsg(LayoutMsg::Increase)
        },
        Keybinding {
          mods: 0,
          key: "h".to_string(),
          cmd: Cmd::SendLayoutMsg(LayoutMsg::Decrease)
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "c".to_string(),
          cmd: Cmd::Exit
        },
        Keybinding {
          mods: MOD_SHIFT,
          key: "x".to_string(),
          cmd: Cmd::Reload
        }
      ],
      manage_hooks: Vec::new(),
      log_hook: None
    };

    for i in 1..10 {
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
          key: key.to_string(),
          cmd: Cmd::SwitchScreen(i)
        });

      config.keybindings.push(
        Keybinding {
          mods: MOD_SHIFT,
          key: key.to_string(),
          cmd: Cmd::MoveToScreen(i)
        });
    }

    config
  }
}

impl Config {
  fn compile() -> Result<(),String> {
    let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/.build"));
    if !dst.exists() {
      create_dir(dst).unwrap_or_else(|e| panic!("Failed to create config build directory: {}", e));
    }

    if !dst.join("Cargo.toml").exists() {
      let mut f = File::create(dst.join("Cargo.toml")).unwrap();
      f.write_all(
b"[project]
name = \"config\"
version = \"0.0.1\"
authors = [\"xr3wm\"]

[dependencies.xr3wm]
git = \"https://github.com/tsurai/xr3wm.git\"

[lib]
name = \"config\"
path = \"../config.rs\"
crate-type = [\"dylib\"]").unwrap();
    }

    let output = Command::new("cargo").arg("build").current_dir(dst).output().unwrap_or_else(|e| panic!("Failed to run cargo: {}", e));
    if output.status.success() {
      Ok(())
    } else {
      unsafe {
        Err(String::from_utf8_unchecked(output.stderr))
      }
    }
  }

  pub fn load() -> Config {
    let mut cfg: Config = Default::default();
    match Config::compile() {
      Ok(_) => {
        match DynamicLibrary::open(Some(&Path::new(concat!(env!("HOME"), "/.xr3wm/.build/target/debug/libconfig.so")))) {
          Ok(lib) => {
            unsafe {
              match lib.symbol("config") {
                Ok(symbol) => {
                  let f = mem::transmute::<*mut u8, extern fn(&mut Config)>(symbol);
                  cfg.lib = Some(lib);
                  f(&mut cfg);
                },
                Err(e) => error!("Failed to load symbol: {}", e)
              }
            }
          },
          Err(e) => error!("Failed to load libconfig: {}", e)
        }
      },
      Err(e) => error!("Failed to compile config: {}", e)
    }
    cfg
  }
}
