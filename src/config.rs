#![allow(unused)]

use crate::keycode::*;
use crate::workspace::WorkspaceConfig;
use crate::workspaces::Workspaces;
use crate::xlib_window_system::XlibWindowSystem;
use crate::commands::{Cmd, ManageHook};
use crate::statusbar::Statusbar;
use crate::layout::*;
use std::iter::FromIterator;
use std::default::Default;
use std::io::{self, Write};
use std::path::Path;
use std::fs::{File, create_dir};
use std::process::{Command, Child, Stdio};
use std::collections::HashMap;
use libloading::{Library, Symbol};
use failure::*;

pub struct WorkspaceConfigList(Vec<WorkspaceConfig>);

impl Default for WorkspaceConfigList {
    fn default() -> WorkspaceConfigList {
        (1usize..10)
            .map(|idx| {
                WorkspaceConfig {
                    tag: idx.to_string(),
                    screen: 0,
                    layout: Strut::new(Tall::new(1, 0.5, 0.05)),
                }
            })
            .collect::<Vec<WorkspaceConfig>>()
            .into()
    }
}

impl From<Vec<WorkspaceConfig>> for WorkspaceConfigList {
    fn from(list: Vec<WorkspaceConfig>) -> Self {
        WorkspaceConfigList(list)
    }
}

pub struct WorkspaceInfo {
    pub id: usize,
    pub tag: String,
    pub screen: usize,
    pub current: bool,
    pub visible: bool,
    pub urgent: bool,
}

pub struct LogInfo {
    pub workspaces: Vec<WorkspaceInfo>,
    pub layout_name: String,
    pub window_title: String,
}

pub struct Config {
    pub mod_key: u8,
    pub border_width: u32,
    pub border_color: u32,
    pub border_focus_color: u32,
    pub border_urgent_color: u32,
    pub greedy_view: bool,
    pub keybindings: HashMap<Keybinding, Cmd>,
    pub manage_hooks: Vec<ManageHook>,
    pub statusbar: Option<Statusbar>,
}

impl Default for Config {
    fn default() -> Config {
        let mut config = Config {
            mod_key: MOD_4,
            border_width: 2,
            border_color: 0x002e_2e2e,
            border_focus_color: 0x002a_82e6,
            border_urgent_color: 0x00ff_0000,
            greedy_view: false,
            keybindings: vec![(
                            Keybinding {
                                mods: 0,
                                key: "Return".to_string()
                            },
                            Cmd::Exec("xterm -u8".to_string())
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "d".to_string(),
                            },
                            Cmd::Exec("dmenu_run".to_string())
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "q".to_string(),
                            },
                            Cmd::KillClient
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "j".to_string(),
                            },
                            Cmd::FocusDown
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "k".to_string(),
                            },
                            Cmd::FocusUp
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "m".to_string(),
                            },
                            Cmd::FocusMaster
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "u".to_string(),
                            },
                            Cmd::FocusParentDown
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "i".to_string(),
                            },
                            Cmd::FocusParentUp
                        ),(
                            Keybinding {
                                mods: MOD_CONTROL,
                                key: "m".to_string(),
                            },
                            Cmd::FocusParentMaster
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "j".to_string(),
                            },
                            Cmd::SwapDown
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "k".to_string(),
                            },
                            Cmd::SwapUp
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "Return".to_string(),
                            },
                            Cmd::SwapMaster
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "u".to_string(),
                            },
                            Cmd::SwapParentDown
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "i".to_string(),
                            },
                            Cmd::SwapParentUp
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "comma".to_string(),
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::IncreaseMaster)
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "period".to_string()
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::DecreaseMaster)
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "l".to_string()
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::Increase)
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "h".to_string()
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::Decrease)
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "space".to_string()
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::NextLayout)
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "space".to_string()
                            },
                            Cmd::SendLayoutMsg(LayoutMsg::PrevLayout)
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "v".to_string(),
                            },
                            Cmd::NestLayout(Box::new(|| Vertical::new()))
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "b".to_string(),
                            },
                            Cmd::NestLayout(Box::new(|| Horizontal::new()))
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "c".to_string(),
                            },
                            Cmd::Exit
                        ),(
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "x".to_string(),
                            },
                            Cmd::Reload
                        )]
                        .drain(0..).collect(),
            manage_hooks: Vec::new(),
            statusbar: None,
        };

        for i in 1..10 {
            config.keybindings.insert(Keybinding {
                    mods: 0,
                    key: i.to_string(),
                },
                Cmd::SwitchWorkspace(i)
            );

            config.keybindings.insert(Keybinding {
                    mods: MOD_SHIFT,
                    key: i.to_string()
                },
                Cmd::MoveToWorkspace(i)
            );
        }

        for &(i, key) in vec![(1, "w"), (2, "e"), (3, "r")].iter() {
            config.keybindings.insert(Keybinding {
                    mods: 0,
                    key: key.to_string()
                },
                Cmd::SwitchScreen(i)
            );

            config.keybindings.insert(Keybinding {
                    mods: MOD_SHIFT,
                    key: key.to_string()
                },
                Cmd::MoveToScreen(i)
            );
        }

        config
    }
}

impl Config {
    fn compile() -> Result<(), Error> {
        let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/.build"));
        if !dst.exists() {
            create_dir(dst)
                .context("failed to create config build directory")?
        }

        if !dst.join("Cargo.toml").exists() {
            let mut f = File::create(dst.join("Cargo.toml"))
                .context("failed to create Cargo.toml")?;

            f.write_all(b"[project]
name = \"config\"
version = \"0.0.1\"
authors = [\"xr3wm\"]

[dependencies.xr3wm]
git = \"https://github.com/tsurai/xr3wm.git\"

[lib]
name = \"config\"
path = \"../config.rs\"
crate-type = [\"dylib\"]")
                .context("failed to write Cargo.toml")?;
        }

        let output = Command::new("cargo")
            .arg("build")
            .current_dir(dst)
            .output()
            .context("failed to execute cargo")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8(output.stderr)
                .context("failed to convert cargo stderr to UTF-8")?;
            bail!(stderr_msg)
        }

        Ok(())
    }

    pub fn load() -> Result<(Config, Vec<WorkspaceConfig>), Error> {
        Config::compile()
            .context("failed to compile config")?;

        let lib: Library = ::libloading::os::unix::Library::open(Some(concat!(env!("HOME"), "/.xr3wm/.build/target/debug/libconfig.so")), libc::RTLD_NOW | libc::RTLD_NODELETE)
            .context("failed to load libconfig")?.into();

        let fn_configure_wm: Symbol<extern fn() -> Config> = unsafe { lib.get(b"configure_wm") }
            .context("failed to get symbol")?;

        let fn_configure_ws: Symbol<extern fn() -> Vec<WorkspaceConfig>> = unsafe { lib.get(b"configure_workspaces") }
            .context("failed to get symbol")?;

        let cfg_wm = fn_configure_wm();
        let cfg_ws = fn_configure_ws();

        Ok((cfg_wm, cfg_ws))
    }
}
