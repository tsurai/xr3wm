use std::io::process::Command;
use config::Config;
use xlib_window_system::XlibWindowSystem;
use workspaces::{Workspaces, MoveOp};

pub enum Cmd {
  Exec(String),
  SwitchWorkspace(uint),
  MoveToWorkspace(uint),
  KillClient,
  FocusUp,
  FocusDown,
  FocusMaster,
  SwapUp,
  SwapDown,
  SwapMaster,
}

impl Cmd {
  pub fn run(&self, ws: &XlibWindowSystem, workspaces: &mut Workspaces, config: &Config) {
    match *self {
      Cmd::Exec(ref cmd) => {
        exec(cmd.clone());
      },
      Cmd::SwitchWorkspace(index) => {
        workspaces.change_to(ws, config, index);
      },
      Cmd::MoveToWorkspace(index) => {
        workspaces.move_window_to(ws, config, index);
      },
      Cmd::KillClient => {
        ws.kill_window(workspaces.get_current().get_focused_window());
      },
      Cmd::FocusUp => {
        workspaces.get_current().move_focus(ws, config, MoveOp::Up);
      },
      Cmd::FocusDown => {
        workspaces.get_current().move_focus(ws, config, MoveOp::Down);
      },
      Cmd::FocusMaster => {
        workspaces.get_current().move_focus(ws, config, MoveOp::Swap);
      },
      Cmd::SwapUp => {
        workspaces.get_current().move_window(ws, config, MoveOp::Up);
      },
      Cmd::SwapDown => {
        workspaces.get_current().move_window(ws, config, MoveOp::Down);
      },
      Cmd::SwapMaster => {
        workspaces.get_current().move_window(ws, config, MoveOp::Swap);
      }
    }
  }
}

fn exec(cmd: String) {
  spawn(proc() {
    match Command::new(cmd.clone()).detached().spawn() {
      Ok(_) => (),
      _ => panic!("failed to start \"{}\"", cmd)
    }
  });
}