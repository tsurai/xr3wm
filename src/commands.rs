#![allow(dead_code)]

extern crate libc;

use crate::config::Config;
use crate::layout::{Layout, LayoutMsg};
use crate::xlib_window_system::XlibWindowSystem;
use crate::workspaces::Workspaces;
use crate::workspace::MoveOp;
use self::libc::execvp;
use std::{env, thread};
use std::ptr::null;
use std::ffi::CString;
use std::process::Command;
use std::path::Path;
use std::fs::OpenOptions;
use x11::xlib::Window;
use anyhow::{bail, Context, Result};

pub enum Cmd {
    Exec(String),
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
    pub fn call(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces, config: &Config) -> Result<()> {
        match self {
            Cmd::Exec(ref cmd) => {
                debug!("Cmd::Exec: {}", cmd);
                exec(cmd.clone());
            }
            Cmd::SwitchWorkspace(index) => {
                debug!("Cmd::SwitchWorkspace: {}", index);
                workspaces.switch_to(ws, config, index - 1, true);
            }
            Cmd::SwitchScreen(screen) => {
                debug!("Cmd::SwitchScreen: {}", screen);
                workspaces.switch_to_screen(ws, config, screen - 1);
            }
            Cmd::MoveToWorkspace(index) => {
                debug!("Cmd::MoveToWorkspace: {}", index);
                workspaces.move_window_to(ws, config, index - 1);
            }
            Cmd::MoveToScreen(screen) => {
                debug!("Cmd::MoveToScreen: {}", screen);
                workspaces.move_window_to_screen(ws, config, screen - 1);
            }
            Cmd::SendLayoutMsg(ref msg) => {
                debug!("Cmd::SendLayoutMsg::{:?}", msg);
                workspaces.current_mut().send_layout_message(msg.clone());
                workspaces.current().redraw(ws, config);
            }
            Cmd::NestLayout(layout_fn) => {
                let layout = layout_fn();
                debug!("Cmd::NestLayout: {}", layout.name());
                workspaces.current_mut().nest_layout(layout);
            }
            Cmd::Reload => {
                debug!("Cmd::Reload");
                reload(workspaces)
                    .context("failed to reload xr3wm")?;
            }
            Cmd::Exit => {
                debug!("Cmd::Exit");
                ws.close();
            }
            Cmd::KillClient => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::KillClient: {}", window);
                    ws.kill_window(window);
                }
            }
            Cmd::FocusUp => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusUp: {}", window);
                    workspaces.current_mut().move_focus(ws, config, MoveOp::Up);
                }
            }
            Cmd::FocusDown => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusDown: {}", window);
                    workspaces.current_mut().move_focus(ws, config, MoveOp::Down);
                }
            }
            Cmd::FocusMaster => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusMaster: {}", window);
                    workspaces.current_mut().move_focus(ws, config, MoveOp::Swap);
                }
            }
            Cmd::FocusParentUp => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusParentUp: {}", window);
                    workspaces.current_mut().move_parent_focus(ws, config, MoveOp::Up);
                }
            }
            Cmd::FocusParentDown => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusParentDown: {}", window);
                    workspaces.current_mut().move_parent_focus(ws, config, MoveOp::Down);
                }
            }
            Cmd::FocusParentMaster => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::FocusParentMaster: {}", window);
                    workspaces.current_mut().move_parent_focus(ws, config, MoveOp::Swap);
                }
            }
            Cmd::SwapUp => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapUp: {}", window);
                    workspaces.current_mut().move_window(ws, config, MoveOp::Up);
                }
            }
            Cmd::SwapDown => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapDown: {}", window);
                    workspaces.current_mut().move_window(ws, config, MoveOp::Down);
                }
            }
            Cmd::SwapMaster => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapMaster: {}", window);
                    workspaces.current_mut().move_window(ws, config, MoveOp::Swap);
                }
            }
            Cmd::SwapParentUp => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapParentUp: {}", window);
                    workspaces.current_mut().move_parent_window(ws, config, MoveOp::Up);
                }
            }
            Cmd::SwapParentDown => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapParentDown: {}", window);
                    workspaces.current_mut().move_parent_window(ws, config, MoveOp::Down);
                }
            }
            Cmd::SwapParentMaster => {
                if let Some(window) = workspaces.current().focused_window() {
                    debug!("Cmd::SwapParentMaster: {}", window);
                    workspaces.current_mut().move_parent_window(ws, config, MoveOp::Swap);
                }
            }
        }
        Ok(())
    }
}

fn reload(workspaces: &mut Workspaces) -> Result<()> {
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

    serde_cbor::to_writer(file, &workspaces)
        .context("failed to serialize workspace states")?;

    let mut args: Vec<*const libc::c_char> = env::args()
        .filter_map(|x| CString::new(x).ok())
        .map(|x| x.into_raw() as *const libc::c_char)
        .collect();
    args.push(null());

    unsafe {
        execvp(args[0] as *const libc::c_char, args.as_ptr());
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
                ws: &XlibWindowSystem,
                workspaces: &mut Workspaces,
                config: &Config,
                window: Window) {
        match *self {
            CmdManage::Move(index) => {
                debug!("CmdManage::Move: {}, {}", window, index);
                workspaces.add_window(Some(index - 1), ws, config, window);
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

fn exec(cmd: String) {
    thread::spawn(move || {
        let args: Vec<&str> = cmd[..].split(' ').collect();

        if !args.is_empty() {
            let mut cmd = Command::new(args[0]);

            if args.len() > 1 {
                cmd.args(&args[1..]);
            }

            match cmd.output() {
                Ok(_) => (),
                _ => panic!("failed to start \"{:?}\"", cmd),
            }
        }
    });
}
