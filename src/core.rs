#[macro_use]

extern crate log;
extern crate dylib;
extern crate xlib;
extern crate xinerama;

pub mod core {
  pub mod commands {
    pub use ::commands::{Cmd, CmdManage, ManageHook, LogInfo, LogHook, CmdLogHook};
  }

  pub mod keycode {
    pub use ::keycode::*;
  }

  pub mod layout {
    pub use ::layout::*;
  }

  pub use ::config::Config;
  pub use ::workspaces::WorkspaceConfig;
}

mod xlib_window_system;
mod config;
mod workspaces;
mod commands;
mod keycode;
mod layout;
