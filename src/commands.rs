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
use std::process::Command;
use std::path::Path;
use std::fs::OpenOptions;
use x11::xlib::Window;
use anyhow::{bail, Context, Result};

pub enum Cmd {
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
            Cmd::Exec(ref cmd, ref args) => {
                debug!("Cmd::Exec: {} {:?}", cmd, args);
                exec(cmd.clone(), args.clone());
            }
            Cmd::SwitchWorkspace(index) => {
                debug!("Cmd::SwitchWorkspace: {}", index);
                state.switch_to(xws, config, index - 1, true);
            }
            Cmd::SwitchScreen(screen) => {
                debug!("Cmd::SwitchScreen: {}", screen);
                state.switch_to_screen(xws, config, screen - 1);
            }
            Cmd::MoveToWorkspace(index) => {
                debug!("Cmd::MoveToWorkspace: {}", index);
                state.move_window_to(xws, config, index - 1);
            }
            Cmd::MoveToScreen(screen) => {
                debug!("Cmd::MoveToScreen: {}", screen);
                state.move_window_to_screen(xws, config, screen - 1);
            }
            Cmd::SendLayoutMsg(ref msg) => {
                debug!("Cmd::SendLayoutMsg::{:?}", msg);
                state.current_ws_mut().send_layout_message(msg.clone());
                state.current_ws().redraw(xws, config);
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
                    debug!("Cmd::KillClient: {}", window);
                    xws.kill_window(window);
                }
            }
            Cmd::FocusUp => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusUp: {}", window);
                    state.current_ws_mut().move_focus(xws, config, MoveOp::Up);
                }
            }
            Cmd::FocusDown => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusDown: {}", window);
                    state.current_ws_mut().move_focus(xws, config, MoveOp::Down);
                }
            }
            Cmd::FocusMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusMaster: {}", window);
                    state.current_ws_mut().move_focus(xws, config, MoveOp::Swap);
                }
            }
            Cmd::FocusParentUp => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusParentUp: {}", window);
                    state.current_ws_mut().move_parent_focus(xws, config, MoveOp::Up);
                }
            }
            Cmd::FocusParentDown => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusParentDown: {}", window);
                    state.current_ws_mut().move_parent_focus(xws, config, MoveOp::Down);
                }
            }
            Cmd::FocusParentMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::FocusParentMaster: {}", window);
                    state.current_ws_mut().move_parent_focus(xws, config, MoveOp::Swap);
                }
            }
            Cmd::SwapUp => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapUp: {}", window);
                    state.current_ws_mut().move_window(xws, config, MoveOp::Up);
                }
            }
            Cmd::SwapDown => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapDown: {}", window);
                    state.current_ws_mut().move_window(xws, config, MoveOp::Down);
                }
            }
            Cmd::SwapMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapMaster: {}", window);
                    state.current_ws_mut().move_window(xws, config, MoveOp::Swap);
                }
            }
            Cmd::SwapParentUp => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapParentUp: {}", window);
                    state.current_ws_mut().move_parent_window(xws, config, MoveOp::Up);
                }
            }
            Cmd::SwapParentDown => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapParentDown: {}", window);
                    state.current_ws_mut().move_parent_window(xws, config, MoveOp::Down);
                }
            }
            Cmd::SwapParentMaster => {
                if let Some(window) = state.current_ws().focused_window() {
                    debug!("Cmd::SwapParentMaster: {}", window);
                    state.current_ws_mut().move_parent_window(xws, config, MoveOp::Swap);
                }
            }
        }
        Ok(())
    }
}

fn reload(state: &WmState) -> Result<()> {
    info!("recompiling...");

    let config_build_dir = concat!(env!("HOME"), "/.xr3wm/.build");
    let mut cmd = Command::new("cargo");

    let output = cmd.current_dir(&config_build_dir)
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

    let path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));

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
        let mut cmd = Command::new(cmd);

        if !args.is_empty() {
            cmd.args(&args);
        }


        match cmd.envs(env::vars()).spawn() {
            Ok(_) => (),
            _ => panic!("failed to start \"{:?}\"", cmd),
        }
    }
}
