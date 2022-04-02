#[macro_use]
extern crate log;

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

    pub use crate::config::{Config, LogInfo};
    pub use crate::statusbar::Statusbar;
    pub use crate::workspace::WorkspaceConfig;
}

mod xlib_window_system;
mod config;
mod state;
mod workspace;
mod commands;
mod keycode;
mod stack;
mod statusbar;
mod layout;
