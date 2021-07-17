#![allow(unused)]

use std::iter::FromIterator;
use std::default::Default;
use std::io::{self, Write};
use std::path::Path;
use std::fs::{File, create_dir};
use std::process::{Command, Child, Stdio};
use std::collections::HashMap;
use failure::*;
use layout::*;
use keycode::*;
use workspaces::{Workspaces, WorkspaceConfig};
use xlib_window_system::XlibWindowSystem;
use commands::{Cmd, ManageHook};
use libloading::{Library, Symbol};

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

pub struct Statusbar {
    child: Option<Child>,
    executable: String,
    args: Option<Vec<String>>,
    fn_format: Box<dyn Fn(LogInfo) -> String>,
}

impl Statusbar {
    pub fn new(executable: String,
               args: Option<Vec<String>>,
               fn_format: Box<dyn Fn(LogInfo) -> String>)
               -> Statusbar {
        Statusbar {
            child: None,
            executable,
            args,
            fn_format,
        }
    }

    pub fn xmobar() -> Statusbar {
        Statusbar::new("xmobar".to_string(),
                       None,
                       Box::new(move |info: LogInfo| -> String {
            let workspaces = info.workspaces
                .iter()
                .map(|x| {
                    let (fg, bg) = if x.current {
                        ("#00ff00", "#000000")
                    } else if x.visible {
                        ("#009900", "#000000")
                    } else if x.urgent {
                        ("#ff0000", "#000000")
                    } else {
                        ("#ffffff", "#000000")
                    };
                    format!("<fc={},{}>[{}]</fc>", fg, bg, x.tag)
                })
                .collect::<Vec<String>>()
                .join(" ");

            format!("{} | {} | {}\n",
                    workspaces,
                    info.layout_name,
                    info.window_title)
        }))
    }

    pub fn start(&mut self) -> Result<(), Error> {
        if self.child.is_some() {
            bail!(format!("'{}' is already running", self.executable));
        }

        debug!("starting statusbar {}", self.executable);
        let mut cmd = Command::new(self.executable.clone());

        if self.args.is_some() {
            cmd.args(self.args.clone().expect("args missing").as_slice());
        }

        self.child = Some(cmd.stdin(Stdio::piped()).spawn()
            .context(format!("failed to execute '{}'", self.executable))?);

        Ok(())
    }

    pub fn update(&mut self, ws: &XlibWindowSystem, workspaces: &Workspaces) -> Result<(), Error> {
        if self.child.is_none() {
            return Ok(());
        }

        let output = (self.fn_format)(LogInfo {
            workspaces: workspaces.all()
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    WorkspaceInfo {
                        id: i,
                        tag: x.get_tag().to_string(),
                        screen: 0,
                        current: i == workspaces.get_index(),
                        visible: x.is_visible(),
                        urgent: x.is_urgent(),
                    }
                })
                .collect(),
            layout_name: workspaces.current().get_layout().name(),
            window_title: ws.get_window_title(workspaces.current().focused_window()),
        });

        let stdin = self.child.as_mut()
            .ok_or_else(|| err_msg("failed to get statusbar process"))?
            .stdin
            .as_mut()
            .ok_or_else(|| err_msg("failed to get statusbar stdin"))?;

        stdin.write_all(output.as_bytes())
            .context("failed to write to statusbar stdin")?;

        Ok(())
    }
}

pub struct Config {
    pub workspaces: Vec<WorkspaceConfig>,
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
            workspaces: (1usize..10)
                .map(|idx| {
                    WorkspaceConfig {
                        tag: idx.to_string(),
                        screen: 0,
                        layout: Strut::new(Tall::new(1, 0.5, 0.05)),
                    }
                })
                .collect(),
            mod_key: MOD_4,
            border_width: 2,
            border_color: 0x002e_2e2e,
            border_focus_color: 0x002a_82e6,
            border_urgent_color: 0x00ff_0000,
            greedy_view: false,
            keybindings: HashMap::from_iter(vec![(
                            Keybinding {
                                mods: 0,
                                key: "Return".to_string()
                            },
                            Cmd::Exec("xterm -u8".to_string())
                        ),
                        (
                            Keybinding {
                                mods: 0,
                                key: "d".to_string(),
                            },
                            Cmd::Exec("dmenu_run".to_string())
                        ),
                        (
                            Keybinding {
                                mods: MOD_SHIFT,
                                key: "q".to_string(),
                            },
                            Cmd::KillClient
                        ),
                        (
                            Keybinding {
                                mods: 0,
                                key: "j".to_string(),
                            },
                            Cmd::FocusDown
                        ),
                        (
                            Keybinding {
                                mods: 0,
                                key: "k".to_string(),
                            },
                            Cmd::FocusUp
                        ),
                        (
                            Keybinding {
                                mods: 0,
                                key: "m".to_string(),
                            },
                            Cmd::FocusMaster
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
                        .drain(0..)),
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

    pub fn load() -> Result<Config, Error> {
        let mut cfg: Config = Default::default();

        Config::compile()
            .context("failed to compile config")?;

        let lib: Library = ::libloading::os::unix::Library::open(Some(concat!(env!("HOME"), "/.xr3wm/.build/target/debug/libconfig.so")), libc::RTLD_NOW | libc::RTLD_NODELETE)
            .context("failed to load libconfig")?.into();

        let func: Symbol<extern fn(&mut Config)> = unsafe { lib.get(b"configure") }
            .context("failed to get symbol")?;

        func(&mut cfg);

        Ok(cfg)
    }
}
