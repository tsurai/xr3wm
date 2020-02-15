#![allow(dead_code)]

extern crate libc;

use self::libc::execvp;
use std::{env, thread};
use std::ptr::null;
use std::ffi::CString;
use std::process::Command;
use std::io::prelude::*;
use std::path::Path;
use std::fs::OpenOptions;
use config::Config;
use layout::LayoutMsg;
use xlib_window_system::XlibWindowSystem;
use workspaces::{Workspaces, MoveOp};
use xlib::Window;
use failure::*;

pub enum Cmd {
    Exec(String),
    SwitchWorkspace(usize),
    SwitchScreen(usize),
    MoveToWorkspace(usize),
    MoveToScreen(usize),
    SendLayoutMsg(LayoutMsg),
    Reload,
    Exit,
    KillClient,
    FocusUp,
    FocusDown,
    FocusMaster,
    SwapUp,
    SwapDown,
    SwapMaster,
}

impl Cmd {
    pub fn call(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces, config: &Config) -> Result<(), Error> {
        match *self {
            Cmd::Exec(ref cmd) => {
                debug!("Cmd::Exec: {}", cmd);
                exec(cmd.clone());
            }
            Cmd::SwitchWorkspace(index) => {
                debug!("Cmd::SwitchWorkspace: {}", index);
                workspaces.switch_to(ws, config, index - 1);
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
                debug!("Cmd::KillClient: {}",
                       workspaces.current_mut().focused_window());
                ws.kill_window(workspaces.current_mut().focused_window());
            }
            Cmd::FocusUp => {
                debug!("Cmd::FocusUp: {}", workspaces.current().focused_window());
                workspaces.current_mut().move_focus(ws, config, MoveOp::Up);
            }
            Cmd::FocusDown => {
                debug!("Cmd::FocusDown: {}", workspaces.current().focused_window());
                workspaces.current_mut().move_focus(ws, config, MoveOp::Down);
            }
            Cmd::FocusMaster => {
                debug!("Cmd::FocusMaster: {}",
                       workspaces.current().focused_window());
                workspaces.current_mut().move_focus(ws, config, MoveOp::Swap);
            }
            Cmd::SwapUp => {
                debug!("Cmd::SwapUp: {}", workspaces.current().focused_window());
                workspaces.current_mut().move_window(ws, config, MoveOp::Up);
            }
            Cmd::SwapDown => {
                debug!("Cmd::SwapDown: {}", workspaces.current().focused_window());
                workspaces.current_mut().move_window(ws, config, MoveOp::Down);
            }
            Cmd::SwapMaster => {
                debug!("Cmd::SwapMaster: {}", workspaces.current().focused_window());
                workspaces.current_mut().move_window(ws, config, MoveOp::Swap);
            }
        }
        Ok(())
    }
}

fn reload(workspaces: &mut Workspaces) -> Result<(), Error> {
    let curr_exe = env::current_exe()
        .context("failed to get executable path")?;
    let filename = curr_exe.file_name()
        .ok_or_else(|| err_msg("failed to get executable filename"))?
        .to_str()
        .ok_or_else(|| err_msg("failed to convert filename to UTF-8"))?;

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
        bail!(format_err!("failed to recompile: {}", stderr_msg))
    }

    debug!("Cmd::Reload: restarting xr3wm...");

    let filename = CString::new(filename.as_bytes())
        .context("failed to convert filename to CString")?;
    let args : &mut [*const i8; 2] = &mut [
        filename.as_ptr() as *const i8,
        null()
    ];

    let path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));

    // save current workspace state to load on restart
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .context("failed to open workspace state tmp file")?;
    file.write_all(workspaces.serialize().as_bytes())
        .context("failed to save workspace state")?;
    file.flush()
        .context("failed to flush workspace tmp file")?;

    let curr_exe_str = CString::new(curr_exe.to_str()
        .ok_or_else(|| err_msg("failed to convert executable path to UTF-8"))?
        .as_bytes())
        .context("failed to convert executable path to CString")?;

    unsafe {
        execvp(curr_exe_str.as_ptr() as *const i8, args.as_mut_ptr());
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
                if let Some(parent) = ws.transient_for(window) {
                    if let Some(workspace) = workspaces.find_window(parent) {
                        workspace.add_window(ws, config, window);
                        workspace.focus_window(ws, config, window);
                    }
                } else {
                    debug!("CmdManage::Move: {}, {}", window, index);
                    workspaces.get_mut(index - 1).add_window(ws, config, window);
                    workspaces.get_mut(index - 1).focus_window(ws, config, window);
                }
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
