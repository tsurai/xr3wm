#[macro_use]
extern crate log;
extern crate libloading;
extern crate xlib;
extern crate xinerama;
extern crate libc;
extern crate failure;

pub mod core {
    pub mod commands {
        pub use ::commands::{Cmd, CmdManage, ManageHook};
    }

    pub mod keycode {
        pub use ::keycode::*;
    }

    pub mod layout {
        pub use ::layout::*;
    }

    pub use ::config::{Config, Statusbar, LogInfo};
    pub use ::workspaces::WorkspaceConfig;
}

mod xlib_window_system;
mod config;
mod workspaces;
mod commands;
mod keycode;
mod layout;
