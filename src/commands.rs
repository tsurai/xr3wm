#![allow(dead_code, unused_must_use)]

extern crate libc;

use self::libc::funcs::posix88::unistd::execvp;
use std::thread;
use std::ptr::null;
use std::os;
use std::ffi::CString;
use std::process::Command;
use std::old_io::fs::PathExtensions;
use std::old_io::{File, Open, Write};
use std::fs;
use config::Config;
use layout::LayoutMsg;
use xlib_window_system::XlibWindowSystem;
use workspaces::{Workspaces, MoveOp};
use xlib::Window;

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
  pub fn call<'a>(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces<'a>, config: &Config) {
    match *self {
      Cmd::Exec(ref cmd) => {
        debug!("Cmd::Exec: {}", cmd);
        exec(cmd.clone());
      },
      Cmd::SwitchWorkspace(index) => {
        debug!("Cmd::SwitchWorkspace: {}", index);
        workspaces.switch_to(ws, config, index - 1);
      },
      Cmd::SwitchScreen(screen) => {
        debug!("Cmd::SwitchScreen: {}", screen);
        workspaces.switch_to_screen(ws, config, screen - 1);
      },
      Cmd::MoveToWorkspace(index) => {
        debug!("Cmd::MoveToWorkspace: {}", index);
        workspaces.move_window_to(ws, config, index - 1);
      },
      Cmd::MoveToScreen(screen) => {
        debug!("Cmd::MoveToScreen: {}", screen);
        workspaces.move_window_to_screen(ws, config, screen - 1);
      },
      Cmd::SendLayoutMsg(ref msg) => {
        debug!("Cmd::SendLayoutMsg::{:?}", msg);
        workspaces.current_mut().get_layout_mut().send_msg(msg.clone());
        workspaces.current().redraw(ws, config);
      },
      Cmd::Reload => {
        let path = os::self_exe_name().unwrap();
        let filename = String::from_str(path.filename_str().unwrap());
        let absolute = String::from_str(path.as_str().unwrap());
        let dir = String::from_str(path.dirname_str().unwrap());

        println!("recompiling...");
        debug!("Cmd::Reload: compiling...");

        let mut cmd = Command::new(&String::from_str("cargo"));
        cmd.current_dir(&Path::new(dir)).arg("build").env("RUST_LOG", "none");

        match cmd.output() {
          Ok(output) => {
            if output.status.success() {
              debug!("Cmd::Reload: restarting... {}", absolute);

              unsafe {
                let mut slice : &mut [*const i8; 2] = &mut [
                  CString::new(filename.as_bytes()).unwrap().as_bytes_with_nul().as_ptr() as *const i8,
                  null()
                ];

                let path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));
                if path.exists() {
                  fs::remove_file(&path);
                }

                let mut file = File::open_mode(&path, Open, Write).unwrap();
                file.write_str(&workspaces.serialize()[..]);
                file.flush();

                execvp(CString::new(absolute.as_bytes()).unwrap().as_bytes_with_nul().as_ptr() as *const i8, slice.as_mut_ptr());
              }
            } else {
              panic!("failed to recompile: '{}'", output.status);
            }
          },
          _ => panic!("failed to start \"{:?}\"", cmd)
        }
      },
      Cmd::Exit => {
        debug!("Cmd::Exit");
        ws.close();
      },
      Cmd::KillClient => {
        debug!("Cmd::KillClient: {}", workspaces.current_mut().focused_window());
        ws.kill_window(workspaces.current_mut().focused_window());
      },
      Cmd::FocusUp => {
        debug!("Cmd::FocusUp: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_focus(ws, config, MoveOp::Up);
      },
      Cmd::FocusDown => {
        debug!("Cmd::FocusDown: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_focus(ws, config, MoveOp::Down);
      },
      Cmd::FocusMaster => {
        debug!("Cmd::FocusMaster: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_focus(ws, config, MoveOp::Swap);
      },
      Cmd::SwapUp => {
        debug!("Cmd::SwapUp: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_window(ws, config, MoveOp::Up);
      },
      Cmd::SwapDown => {
        debug!("Cmd::SwapDown: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_window(ws, config, MoveOp::Down);
      },
      Cmd::SwapMaster => {
        debug!("Cmd::SwapMaster: {}", workspaces.current().focused_window());
        workspaces.current_mut().move_window(ws, config, MoveOp::Swap);
      }
    }
  }
}

pub struct ManageHook {
  pub class_name: String,
  pub cmd: CmdManage
}

pub enum CmdManage {
  Move(usize),
  Float,
  Fullscreen,
  Ignore
}

impl CmdManage {
  pub fn call<'a>(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces<'a>, config: &Config, window: Window) {
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
      },
      CmdManage::Float => {
        debug!("CmdManage::Float");
        unimplemented!()
      },
      CmdManage::Fullscreen => {
        debug!("CmdManage::Fullscreen");
        unimplemented!()
      },
      CmdManage::Ignore => {
        debug!("CmdManage::Ignore");
        unimplemented!()
      }
    }
  }
}

pub enum LogInfo {
  Workspaces(Vec<String>, usize, Vec<usize>, Vec<bool>),
  Title(String),
  Layout(String)
}

pub struct LogHook<'a> {
  pub logs: Vec<CmdLogHook>,
  pub output: Box<Fn(Vec<LogInfo>) -> String + 'a>
}

impl<'a> LogHook<'a> {
  pub fn call<'b>(&mut self, ws: &XlibWindowSystem, workspaces: &Workspaces<'b>) {
    println!("{}", (self.output)(self.logs.iter().map(|x| x.call(ws, workspaces)).collect()));
  }
}

pub enum CmdLogHook {
  Workspaces,
  Title,
  Layout
}

impl CmdLogHook {
  pub fn call<'a>(&self, ws: &XlibWindowSystem, workspaces: &Workspaces<'a>) -> LogInfo {
    match *self {
      CmdLogHook::Workspaces => {
        LogInfo::Workspaces(
          workspaces.all().iter().map(|x| x.get_tag()).collect(),
          workspaces.get_index(),
          workspaces.all().iter().enumerate().filter(|&(_,x)| x.is_visible()).map(|(i,_)| i).collect(),
          workspaces.all().iter().map(|x| x.is_urgent()).collect())
      },
      CmdLogHook::Title => {
        LogInfo::Title(ws.get_window_title(workspaces.current().focused_window()))
      },
      CmdLogHook::Layout => {
        LogInfo::Layout(workspaces.current().get_layout().name())
      }
    }
  }
}

fn exec(cmd: String) {
  thread::spawn(move || {
    let args : Vec<&str> = cmd[..].split(' ').collect();

    if args.len() > 0 {
      let mut cmd = Command::new(args[0]);

      if args.len() > 1 {
        cmd.args(&args[1..]);
      }

      match cmd.output() {
        Ok(_) => (),
        _ => panic!("failed to start \"{:?}\"", cmd)
      }
    }
  });
}
