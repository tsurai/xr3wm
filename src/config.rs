#![allow(unused)]

use crate::keycode::*;
use crate::workspace::WorkspaceConfig;
use crate::state::WmState;
use crate::xlib_window_system::XlibWindowSystem;
use crate::commands::{Cmd, ManageHook};
use crate::statusbar::Statusbar;
use crate::layout::*;
use std::env;
use std::iter::FromIterator;
use std::default::Default;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::process::{Command, Child, Stdio};
use std::collections::HashMap;
use libloading::os::unix::{Library, Symbol};
use anyhow::{bail, Context, Result};

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

pub struct PagerInfo {
    pub workspaces: Vec<WorkspaceInfo>,
    pub layout_names: Vec<String>,
    pub window_title: String,
}

pub struct Config {
    pub path: PathBuf,
    pub mod_key: u8,
    pub border_width: u32,
    pub border_color: u32,
    pub border_focus_color: u32,
    pub border_urgent_color: u32,
    pub greedy_view: bool,
    pub terminal: String,
    pub keybindings: HashMap<Keybinding, Cmd>,
    pub manage_hooks: Vec<ManageHook>,
    pub statusbar: Option<Statusbar>,
}

impl Default for Config {
    fn default() -> Config {
        let mut config = Config {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            mod_key: MOD_4,
            border_width: 2,
            border_color: 0x002e_2e2e,
            border_focus_color: 0x002a_82e6,
            border_urgent_color: 0x00ff_0000,
            greedy_view: false,
            terminal: "xterm".to_string(),
            keybindings: vec![(
                            Keybinding {
                                mods: 0,
                                key: "Return".to_string()
                            },
                            Cmd::SpawnTerminal(vec![])
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "d".to_string(),
                            },
                            Cmd::Exec("dmenu_run".to_string(), vec![])
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
                                mods: MOD_SHIFT,
                                key: "a".to_string()
                            },
                            Cmd::RemoveNested
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "v".to_string(),
                            },
                            Cmd::NestLayout(Box::new(Vertical::new))
                        ),(
                            Keybinding {
                                mods: 0,
                                key: "b".to_string(),
                            },
                            Cmd::NestLayout(Box::new(Horizontal::new))
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
                            Cmd::Reload(vec![])
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
    pub fn get_dir() -> Result<String, env::VarError> {
        let mut cfg_path = None;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" | "-c" => cfg_path = args.next(),
                x if x.starts_with("--config=") => {
                    cfg_path = x.splitn(2, '=').last().map(|x| x.into());
                },
                _ => (),
            }
        }

        cfg_path.ok_or(())
            .or_else(|_| {
                env::var("XDG_CONFIG_HOME")
                    .map(|x| format!("{x}/xr3wm"))
                    .or_else(|_| {
                        env::var("HOME")
                            .map(|x| format!("{x}/.config/xr3wm"))
                    })
            })
    }

    fn create_default_cfg_file(path: &Path) -> Result<()> {
        let mut f = File::create(path.join("config.rs"))
            .context("failed to create config.rs")?;

        f.write_all(b"#![allow(unused_imports)]
extern crate xr3wm;

use std::default::Default;
use xr3wm::core::*;

#[no_mangle]
pub extern fn configure_workspaces() -> Vec<WorkspaceConfig> {
    use layout::*;

    (1usize..10)
        .map(|idx| {
            WorkspaceConfig {
                tag: idx.to_string(),
                screen: 0,
                layout: Strut::new(Choose::new(vec![Tall::new(1, 0.5, 0.05), Rotate::new(Tall::new(1, 0.5, 0.05)), Full::new(false)])),
            }
        })
        .collect()
}

#[no_mangle]
pub extern fn configure_wm() -> Config {
    let mut cfg: Config = Default::default();

    cfg
}")
        .context("failed to write default config file")
    }

    fn create_cargo_toml_file(path: &Path) -> Result<()> {
        let mut f = File::create(path.join("Cargo.toml"))
            .context("failed to create Cargo.toml")?;

        f.write_all(b"[package]
name = \"config\"
version = \"0.0.1\"
authors = [\"xr3wm\"]

[dependencies.xr3wm]
git = \"https://github.com/tsurai/xr3wm.git\"
default-features = true

[lib]
name = \"config\"
path = \"src/config.rs\"
crate-type = [\"dylib\"]")
            .context("failed to write Cargo.toml")
    }

    pub fn compile() -> Result<()> {
        let cfg_dir = Self::get_dir()?;
        let path = Path::new(&cfg_dir).join("src/");
        if !path.exists() {
            fs::create_dir_all(&path)
                .context("failed to create config build directory")?
        }

        if !path.join("config.rs").exists() {
            Self::create_default_cfg_file(&path)?;
        }

        let path = path.join("..");
        if !path.join("Cargo.toml").exists() {
            Self::create_cargo_toml_file(&path)?
        }

        let output = Command::new("cargo")
            .arg("build")
            .current_dir(path)
            .output()
            .context("failed to execute cargo")?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8(output.stderr)
                .context("failed to convert cargo stderr to UTF-8")?;
            bail!("failed to recompile: {}", stderr_msg);
        }

        Ok(())
    }

    pub fn load() -> Result<(Config, Vec<WorkspaceConfig>)> {
        unsafe {
            Config::compile()
                .context("failed to compile config")?;

            let cfg_path = format!("{}/target/debug/libconfig.so", Self::get_dir()?);

            let lib: Library = Library::open(Some(cfg_path), libc::RTLD_NOW | libc::RTLD_NODELETE)
                .context("failed to load libconfig")?;

            let fn_configure_wm: Symbol<extern fn() -> Config> = lib.get(b"configure_wm")
                .context("failed to get symbol")?;

            let fn_configure_ws: Symbol<extern fn() -> Vec<WorkspaceConfig>> = lib.get(b"configure_workspaces")
                .context("failed to get symbol")?;

            let cfg_wm = fn_configure_wm();
            let cfg_ws = fn_configure_ws();

            Ok((cfg_wm, cfg_ws))
        }
    }
}
