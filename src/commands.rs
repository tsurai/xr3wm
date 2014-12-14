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
        ws.kill_window(workspaces.current().get_focused_window());
      },
      Cmd::FocusUp => {
        workspaces.current().move_focus(ws, config, MoveOp::Up);
      },
      Cmd::FocusDown => {
        workspaces.current().move_focus(ws, config, MoveOp::Down);
      },
      Cmd::FocusMaster => {
        workspaces.current().move_focus(ws, config, MoveOp::Swap);
      },
      Cmd::SwapUp => {
        workspaces.current().move_window(ws, config, MoveOp::Up);
      },
      Cmd::SwapDown => {
        workspaces.current().move_window(ws, config, MoveOp::Down);
      },
      Cmd::SwapMaster => {
        workspaces.current().move_window(ws, config, MoveOp::Swap);
      }
    }
  }
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
        workspaces.move_window_to(ws, config, index - 1);
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

fn exec(cmd: String) {
  spawn(proc() {
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
