use std::io::process::Command;
use config::Config;
use xlib_window_system::XlibWindowSystem;
use workspaces::{Workspaces, MoveOp};
use xlib::Window;

pub enum Cmd {
  Exec(String),
  SwitchWorkspace(uint),
  SwitchScreen(uint),
  MoveToWorkspace(uint),
  MoveToScreen(uint),
  KillClient,
  FocusUp,
  FocusDown,
  FocusMaster,
  SwapUp,
  SwapDown,
  SwapMaster,
}

impl Cmd {
  pub fn call(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces, config: &Config) {
    match *self {
      Cmd::Exec(ref cmd) => {
        exec(cmd.clone());
      },
      Cmd::SwitchWorkspace(index) => {
        workspaces.switch_to(ws, config, index - 1);
      },
      Cmd::SwitchScreen(screen) => {
        workspaces.switch_to_screen(ws, config, screen - 1);
      },
      Cmd::MoveToWorkspace(index) => {
        workspaces.move_window_to(ws, config, index - 1);
      },
      Cmd::MoveToScreen(screen) => {
        workspaces.move_window_to_screen(ws, config, screen - 1);
      }
      Cmd::KillClient => {
        ws.kill_window(workspaces.current_mut().get_focused_window());
      },
      Cmd::FocusUp => {
        workspaces.current_mut().move_focus(ws, config, MoveOp::Up);
      },
      Cmd::FocusDown => {
        workspaces.current_mut().move_focus(ws, config, MoveOp::Down);
      },
      Cmd::FocusMaster => {
        workspaces.current_mut().move_focus(ws, config, MoveOp::Swap);
      },
      Cmd::SwapUp => {
        workspaces.current_mut().move_window(ws, config, MoveOp::Up);
      },
      Cmd::SwapDown => {
        workspaces.current_mut().move_window(ws, config, MoveOp::Down);
      },
      Cmd::SwapMaster => {
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
  Move(uint),
  Float,
  Fullscreen,
  Ignore
}

impl CmdManage {
  pub fn call(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces, config: &Config, window: Window) {
    match *self {
      CmdManage::Move(index) => {
        workspaces.get_mut(index - 1).add_window(ws, config, window);
        workspaces.get_mut(index - 1).focus_window(ws, config, window);
      },
      CmdManage::Float => {
        unimplemented!()
      },
      CmdManage::Fullscreen => {
        unimplemented!()
      },
      CmdManage::Ignore => {
        unimplemented!()
      }
    }
  }
}

pub enum LogInfo {
  Workspaces(Vec<String>, uint, Vec<uint>),
  Title(String),
  Layout(String)
}

pub struct LogHook<'a> {
  pub logs: Vec<CmdLogHook>,
  pub output: |Vec<LogInfo>|:'a -> String
}

impl<'a> LogHook<'a> {
  pub fn call(&mut self, ws: &XlibWindowSystem, workspaces: &Workspaces) {
    println!("{}", (self.output)(self.logs.iter().map(|x| x.call(ws, workspaces)).collect()));
  }
}

pub enum CmdLogHook {
  Workspaces,
  Title,
  Layout
}

impl CmdLogHook {
  pub fn call(&self, ws: &XlibWindowSystem, workspaces: &Workspaces) -> LogInfo {
    match *self {
      CmdLogHook::Workspaces => {
        LogInfo::Workspaces(workspaces.all().iter().map(|x| x.get_tag()).collect(), workspaces.get_index(), Vec::new())
      },
      CmdLogHook::Title => {
        LogInfo::Title(ws.get_window_title(workspaces.current().get_focused_window()))
      },
      CmdLogHook::Layout => {
        LogInfo::Layout(workspaces.current().get_layout().name())
      }
    }
  }
}

fn exec(cmd: String) {
  spawn(move || {
    let args : Vec<&str> = cmd.as_slice().split(' ').collect();

    if args.len() > 0 {
      let mut cmd = Command::new(args[0]);

      if args.len() > 1 {
        cmd.args(args.as_slice().slice_from(1));
      }

      match cmd.detached().spawn() {
        Ok(_) => (),
        _ => panic!("failed to start \"{}\"", cmd)
      }
    }

  });
}
