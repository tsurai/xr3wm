#[macro_use]
extern crate log;
extern crate libloading;
extern crate x11;
extern crate libc;
extern crate failure;

pub mod core {
    pub mod commands {
        pub use crate::commands::{Cmd, CmdManage, ManageHook};
    }

    pub mod keycode {
        pub use crate::keycode::*;
    }
    pub use crate::keycode::Keybinding;

    pub mod layout {
        pub use crate::layout::*;
    }

    pub use crate::config::{Config, Statusbar, LogInfo};
    pub use crate::workspace::WorkspaceConfig;
}

mod xlib_window_system;
mod config;
mod workspaces;
mod workspace;
mod commands;
mod keycode;
mod stack;
mod container;
mod layout;
