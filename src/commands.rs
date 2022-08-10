#![allow(dead_code)]

extern crate libc;

use crate::config::Config;
use crate::layout::{Layout, LayoutMsg};
use crate::xlib_window_system::XlibWindowSystem;
use crate::state::WmState;
use crate::workspace::MoveOp;
use self::libc::execvpe;
use std::{env, iter};
use std::ptr::null;
use std::ffi::CString;
use std::process::{Command, Stdio};
use std::path::Path;
use std::fs::OpenOptions;
use x11::xlib::Window;
use anyhow::{bail, Context, Result};

type CustomCmdFn = dyn Fn(&WmState) -> Result<Option<Cmd>, String>;

pub enum Cmd {
    Custom(Box<CustomCmdFn>),
    Exec(String, Vec<String>),
    SwitchWorkspace(usize),
    SwitchScreen(usize),
    MoveToWorkspace(usize),
    MoveToScreen(usize),
    SendLayoutMsg(LayoutMsg),
    NestLayout(Box<dyn Fn() -> Box<dyn Layout>>),
    Reload,
    Exit,
    KillClient,
    FocusParentUp,
    FocusParentDown,
    FocusParentMaster,
    FocusUp,
    FocusDown,
    FocusMaster,
    SwapUp,
    SwapDown,
    SwapMaster,
    SwapParentUp,
    SwapParentDown,
    SwapParentMaster,
}

impl Cmd {
    pub fn call(&self, xws: &XlibWindowSystem, state: &mut WmState, config: &Config) -> Result<()> {
        match self {
            Cmd::Custom(func) => {
                debug!("Cmd::Custom");
                match func(state) {
                    Ok(Some(cmd)) => cmd.call(xws, state, config)?,
                    Ok(None) => (),
                    Err(e) => error!("Cmd::Custom failed: {}", e),
                }
            }
            Cmd::Exec(ref cmd, ref args) => {
                debug!("Cmd::Exec: {} {:?}", cmd, args);
                exec(cmd.clone(), args.clone());
            }
            Cmd::SwitchWorkspace(index) => {
                debug!("Cmd::SwitchWorkspace: {}", index);
                state.switch_to_ws(xws, config, index - 1, true);
            }
            Cmd::SwitchScreen(screen) => {
                debug!("Cmd::SwitchScreen: {}", screen);
                state.switch_to_screen(xws, config, screen - 1);
            }
            Cmd::MoveToWorkspace(index) => {
                debug!("Cmd::MoveToWorkspace: {}", index);
                state.move_window_to_ws(xws, config, index - 1);
            }
            Cmd::MoveToScreen(screen) => {
                debug!("Cmd::MoveToScreen: {}", screen);
                state.move_window_to_screen(xws, config, screen - 1);
            }
            Cmd::SendLayoutMsg(ref msg) => {
                debug!("Cmd::SendLayoutMsg::{:?}", msg);
                state.current_ws_mut().send_layout_message(xws, msg.clone());
                state.current_ws().redraw(xws, config, state.get_screens());
            }
            Cmd::NestLayout(layout_fn) => {
                let layout = layout_fn();
                debug!("Cmd::NestLayout: {}", layout.name());
                state.current_ws_mut().nest_layout(layout);
            }
            Cmd::Reload => {
                debug!("Cmd::Reload");
                reload(state)
                    .context("failed to reload xr3wm")?;
            }
            Cmd::Exit => {
                debug!("Cmd::Exit");
                xws.close();
            }
            Cmd::KillClient => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::KillClient: {:#x}", window);
                    xws.kill_window(window);
                }
            }
            Cmd::FocusUp | Cmd::FocusDown | Cmd::FocusMaster | Cmd::FocusParentUp | Cmd::FocusParentDown | Cmd::FocusParentMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    let workspace = state.current_ws_mut();
                    let new_focus = match self {
                        Cmd::FocusUp => {
                            debug!("Cmd::FocusUp: {:#x}", window);
                            workspace.move_focus(MoveOp::Up)
                        }
                        Cmd::FocusDown => {
                            debug!("Cmd::FocusDown: {:#x}", window);
                            workspace.move_focus(MoveOp::Down)
                        }
                        Cmd::FocusMaster => {
                            debug!("Cmd::FocusMaster: {:#x}", window);
                            workspace.move_focus(MoveOp::Swap)
                        }
                        Cmd::FocusParentUp => {
                            debug!("Cmd::FocusParentUp: {:#x}", window);
                            workspace.move_parent_focus(MoveOp::Up)
                        }
                        Cmd::FocusParentDown => {
                            debug!("Cmd::FocusParentDown: {:#x}", window);
                            workspace.move_parent_focus(MoveOp::Down)
                        }
                        Cmd::FocusParentMaster => {
                            debug!("Cmd::FocusParentMaster: {:#x}", window);
                            workspace.move_parent_focus(MoveOp::Swap)
                        }
                        _ => None
                    };

                    if let Some(window) = new_focus {
                        xws.focus_window(window);
                    }
                }
            },
            Cmd::SwapUp | Cmd::SwapDown | Cmd::SwapMaster | Cmd::SwapParentUp | Cmd::SwapParentDown | Cmd::SwapParentMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    let workspace = state.current_ws_mut();
                    let new_focus = match self {
                        Cmd::SwapUp => {
                            debug!("Cmd::SwapUp: {:#x}", window);
                            workspace.move_window(MoveOp::Up)
                        }
                        Cmd::SwapDown => {
                            debug!("Cmd::SwapDown: {:#x}", window);
                            workspace.move_window(MoveOp::Down)
                        }
                        Cmd::SwapMaster => {
                            debug!("Cmd::SwapMaster: {:#x}", window);
                            workspace.move_window(MoveOp::Swap)
                        }
                        Cmd::SwapParentUp => {
                            debug!("Cmd::SwapParentUp: {:#x}", window);
                            workspace.move_parent_window(MoveOp::Up)
                        }
                        Cmd::SwapParentDown => {
                            debug!("Cmd::SwapParentDown: {:#x}", window);
                            workspace.move_parent_window(MoveOp::Down)
                        }
                        Cmd::SwapParentMaster => {
                            debug!("Cmd::SwapParentMaster: {:#x}", window);
                            workspace.move_parent_window(MoveOp::Swap)
                        }
                        _ => false
                    };

                    if new_focus {
                        state.current_ws().redraw(xws, config, state.get_screens());
                    }
                }
            }
        }
        Ok(())
    }
}

fn reload(state: &WmState) -> Result<()> {
    info!("recompiling...");

    let cfg_dir = Config::get_dir()
        .context("failed to locate config directory")?;
    let mut cmd = Command::new("cargo");

    let output = cmd.current_dir(&format!("{}/.build", cfg_dir))
        .arg("build")
        .env("RUST_LOG", "none")
        .output()
        .context("failed to run cargo")?;

    if !output.status.success() {
        let stderr_msg = String::from_utf8(output.stderr)
            .context("failed to convert cargo stderr to UTF-8")?;
        bail!("failed to recompile: {}", stderr_msg)
    }

    debug!("Cmd::Reload: restarting xr3wm...");

    let path = Path::new(&cfg_dir).join(".state.tmp");

    // save current workspace states to restore on restart
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .context("failed to open workspace state tmp file")?;

    serde_json::to_writer(file, state)
        .context("failed to serialize workspace states")?;

    let args: Vec<*const libc::c_char> = env::args()
        .filter_map(|x| CString::new(x).ok())
        .map(|x| x.into_raw() as *const libc::c_char)
        .chain(iter::once(null()))
        .collect();

    let envs: Vec<*const libc::c_char> = env::vars()
        .filter_map(|(k,v)| CString::new(format!("{}={}", k, v)).ok())
        .map(|x| x.into_raw() as *const libc::c_char)
        .chain(iter::once(null()))
        .collect();

    unsafe {
        execvpe(args[0] as *const libc::c_char, args.as_ptr(), envs.as_ptr());
        // execvp returns only if an error has occurred
        error!("failed to reload: {}", ::std::io::Error::last_os_error());
    }

    Ok(())
}

pub struct ManageHook {
    pub class_name: String,
    pub cmd: CmdManage,
}

pub enum CmdManage {
    Move(usize),
    Float,
    Fullscreen,
    Ignore,
}

impl CmdManage {
    pub fn call(&self,
                xws: &XlibWindowSystem,
                state: &mut WmState,
                config: &Config,
                window: Window) {
        match *self {
            CmdManage::Move(index) => {
                debug!("CmdManage::Move: {}, {}", window, index);
                state.add_window(Some(index - 1), xws, config, window);
            }
            CmdManage::Float => {
                debug!("CmdManage::Float");
                unimplemented!()
            }
            CmdManage::Fullscreen => {
                debug!("CmdManage::Fullscreen");
                unimplemented!()
            }
            CmdManage::Ignore => {
                debug!("CmdManage::Ignore");
                unimplemented!()
            }
        }
    }
}

fn exec(cmd: String, args: Vec<String>) {
    if !cmd.is_empty() {
        std::thread::spawn(move || {
            match Command::new(&cmd)
                .envs(env::vars())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(&args)
                .spawn()
            {
                Ok(mut child) => {
                    child.wait().ok();
                },
                Err(e) => error!("failed to start \"{:?}\": {}", cmd, e),
            }
        });
    }
}
