#[macro_use]
extern crate log;

use anyhow::{Context, Result};
use config::Config;
use state::WmState;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::*;
use std::env;

mod commands;
mod config;
mod ewmh;
mod keycode;
mod layout;
mod stack;
mod state;
mod statusbar;
mod utils;
mod workspace;
mod xlib_window_system;

fn print_version() -> ! {
    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    ::std::process::exit(0);
}

fn print_help() -> ! {
    println!("usage: xr3wm [OPTION]
Xmonad and i3 inspired X11 tiling window manager.

  -c, --config=path config file path
  -h, --help        display this help and exit
  -v, --version     output version information and exit");
    ::std::process::exit(0);
}

fn handle_args() {
    let args = env::args().skip(1);

    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => print_help(),
            "--version" | "-v" => print_version(),
            _ => (),
        }
    }
}

fn run() -> Result<()> {
    handle_args();

    // initialize logging system
    env_logger::init();

    info!("loading config");

    let (mut config, ws_cfg_list) = Config::load()
        .map_err(|e| {
            let error = utils::concat_error_chain(&e);
            utils::xmessage(&format!("failed to load config:\n{}", error))
                .map_err(|e| warn!("failed to run xmessage: {}", e))
                .ok();
            e
        })
        .context("failed to load config")?;

    let xws = &XlibWindowSystem::new();
    xws.init();
    xws.grab_modifier(config.mod_key);

    let mut state = WmState::new(ws_cfg_list, xws)
        .context("failed to create initial wm state")?;

    state.rescreen(xws, &config);

    if let Some(ref mut statusbar) = config.statusbar {
        statusbar.start()
            .context("failed to start statusbar")?;
    }

    ewmh::set_current_desktop(xws, state.get_ws_index());
    ewmh::set_number_of_desktops(xws, state.ws_count());
    ewmh::set_desktop_names(xws, state.all_ws());
    ewmh::set_desktop_viewport(xws, state.all_ws());

    info!("entering event loop");
    run_event_loop(config, xws, state)
}

fn run_event_loop(mut config: Config, xws: &XlibWindowSystem, mut state: WmState) -> Result<()> {
    loop {
        match xws.get_event() {
            XMapRequest(window) => {
                trace!("XMapRequest: {:#x}", window);
                if !state.contains(window) {
                    let mut is_hooked = false;
                    if let Some(class) = xws.get_class_name(window) {
                        for hook in config.manage_hooks.iter() {
                            if hook.class_name == class {
                                hook.cmd.call(xws, &mut state, &config, window);
                                is_hooked = true;
                            }
                        }
                    }

                    if !is_hooked {
                        state.add_window(None, xws, &config, window);
                    }

                    state.focus_window(xws, &config, window);
                }
            }
            XMapNotify(window) => {
                trace!("XMapNotify: {:#x}", window);
                if xws.get_window_strut(window).is_some() {
                    state.add_strut(window);
                    state.redraw(xws, &config);
                }
            }
            XDestroy(window) => {
                trace!("XDestroy: {:#x}", window);
                if state.contains(window) {
                    state.remove_window(xws, &config, window);
                }
            }
            XUnmapNotify(window, send) => {
                trace!("XUnmapNotify: {:#x} {}", window, send);
                if send && state.contains(window) {
                    state.remove_window(xws, &config, window);
                } else if state.try_remove_strut(window) {
                    state.redraw(xws, &config);
                }
            }
            XPropertyNotify(window, atom, is_new_value) => {
                if atom == xws.get_atom("WM_HINTS", true) {
                    if let Some(ws) = state.get_parent_mut(window) {
                        ws.set_urgency(xws.is_urgent(window), window);
                    }
                } else if atom == xws.get_atom("_NET_WM_STRUT_PARTIAL", true) {
                    if is_new_value {
                        state.add_strut(window);
                    } else {
                        state.try_remove_strut(window);
                    }
                    state.redraw(xws, &config);
                }
            }
            XConfigureNotify(_) => {
                trace!("XConfigurationNotify");
                state.rescreen(xws, &config);
            }
            XConfigureRequest(window, changes, mask) => {
                trace!("XConfigureRequest: {:#x}", window);
                let unmanaged = state.is_unmanaged(window) || !state.contains(window);
                xws.configure_window(window, changes, mask, unmanaged);
            }
            XEnterNotify(window, is_root, x, y) => {
                trace!("XEnterNotify: {:#x} {} x: {} y: {}", window, is_root,x ,y);
                if is_root {
                    state.switch_to_ws_at(xws, &config, x, y, false)
                } else {
                    state.focus_window(xws, &config, window);
                }
            }
            XFocusIn(window) => {
                trace!("XFocusIn: {:#x}", window);
                if let Some(idx) = state.find_window(window) {
                    let screens = state.get_screens();
                    if let Some(workspace) = state.get_ws(idx) {
                        workspace.redraw(xws, &config, screens);
                    }
                }
            }
            XButtonPress(window) => {
                state.focus_window(xws, &config, window);
            }
            XKeyPress(_, mods, key) => {
                trace!("XKeyPress: {}, {}", mods, key);
                let mods = mods & !(config.mod_key | 0b10010);

                for (binding, cmd) in config.keybindings.iter() {
                    if binding.mods == mods && binding.key == key {
                        cmd.call(xws, &mut state, &config)
                            .map_err(|e| error!("{}", utils::concat_error_chain(&e)))
                            .ok();
                    }
                }
            }
            _ => {}
        }

        if let Some(ref mut statusbar) = config.statusbar {
            if let Err(e) = statusbar.update(xws, &state) {
                error!("{}", e.context("failed to update statusbar"));
            }
        }
    }

}

fn main() {
    // failure crate boilerplate
    if let Err(e) = run() {
        use std::io::Write;

        let mut stderr = std::io::stderr();

        if log_enabled!(log::Level::Error) {
            error!("{}", e);
        } else {
            writeln!(stderr, "ERROR: {}", e).ok();
        }

        e.chain().skip(1)
            .for_each(|cause| error!("because: {}", cause));

        error!("backtrace: {}", e.backtrace());

        stderr.flush().ok();
        ::std::process::exit(1);
    }
}
