#[macro_use]
extern crate log;

use clap::{Arg, App, ArgMatches};
use clap::AppSettings::*;
use anyhow::{Context, Result};
use config::Config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::*;

mod config;
mod keycode;
mod commands;
mod xlib_window_system;
mod workspaces;
mod workspace;
mod statusbar;
mod stack;
mod layout;
mod utils;

fn process_cli<'a>() -> ArgMatches<'a> {
    App::new("xr3wm")
        .version("0.0.1")
        .author("Cristian Kubis <cristian.kubis@tsunix.de>")
        .about("i3wm inspired tiling window manager")
        .setting(DeriveDisplayOrder)
        .arg(Arg::with_name("verbose")
             .short("v")
             .long("verbose")
             .multiple(true)
             .help("increrases the logging verbosity each use for up to 2 times"))
        .get_matches()
}

// initialization of the logging system
fn init_logger(verbosity: u64, logfile: &str) -> Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
        // set the verbosity of the logging
        .level(match verbosity {
            1 => log::LevelFilter::Debug,
            x if x > 1 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info
        })
        // output everything but errors to stdout
        .chain(
            fern::Dispatch::new()
                .filter(move |metadata| metadata.level() != log::LevelFilter::Error)
                .chain(::std::io::stdout()))
        // output errors to stderr
        .chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Error)
                .chain(::std::io::stderr()))
        // duplicate all logs in a log file
        .chain(fern::log_file(logfile)
            .context("failed to open log file")?)
        .apply()
        .map_err(|e| e.into())
}

fn run() -> Result<()> {
    let matches = process_cli();

    let verbosity = matches.occurrences_of("verbose");

    // initialize logging system
    if let Err(e) = init_logger(verbosity, concat!(env!("HOME"), "/.xr3wm/xr3wm.log")) {
        eprintln!("[ERROR] failed to initialize logging system: {}", e);
        ::std::process::exit(1);
    }

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
    xws.grab_modifier(config.mod_key);

    let mut workspaces = Workspaces::new(ws_cfg_list, xws)
        .context("failed to create workspaces")?;

    workspaces.current_mut().show(xws, &config);

    if let Some(ref mut statusbar) = config.statusbar {
        statusbar.start()
            .context("failed to start statusbar")?;
    }

    info!("entering event loop");
    run_event_loop(config, xws, workspaces)
}

fn run_event_loop(mut config: Config, xws: &XlibWindowSystem, mut workspaces: Workspaces) -> Result<()> {
    loop {
        match xws.get_event() {
            XMapRequest(window) => {
                debug!("XMapRequest: {}", window);
                if !workspaces.contains(window) {
                    let class = xws.get_class_name(window);
                    let mut is_hooked = false;

                    for hook in config.manage_hooks.iter() {
                        if hook.class_name == class {
                            is_hooked = true;
                            hook.cmd.call(xws, &mut workspaces, &config, window);
                        }
                    }

                    if !is_hooked {
                        workspaces.add_window(None, xws, &config, window);
                        workspaces.focus_window(xws, &config, window);
                    }
                }
            }
            XMapNotify(window) => {
                if !workspaces.contains(window) &&
                    xws.get_window_strut(window).is_some()
                {
                    workspaces.redraw_all(xws, &config);
                }
            }
            XDestroy(window) => {
                debug!("XDestroy: {}", window);

                if workspaces.contains(window) {
                    workspaces.remove_window(xws, &config, window);
                }
            }
            XUnmapNotify(window, send) => {
                debug!("XUnmapNotify: {} {}", window, send);

                if send && workspaces.contains(window) {
                    workspaces.remove_window(xws, &config, window);
                }
            }
            XPropertyNotify(window, atom, _) => {
                if atom == xws.get_atom("WM_HINTS", false) {
                    if let Some(idx) = workspaces.find_window(window) {
                        workspaces.get_mut(idx)
                            .set_urgency(xws.is_urgent(window), xws, &config, window);
                    }
                }
            }
            XConfigurationNotify(_) => {
                workspaces.rescreen(xws, &config);
            }
            XConfigurationRequest(window, changes, mask) => {
                let unmanaged = workspaces.is_unmanaged(window) || !workspaces.contains(window);
                xws.configure_window(window, changes, mask, unmanaged);
            }
            XEnterNotify(window) => {
                trace!("XEnterNotify: {}", window);
                workspaces.focus_window(xws, &config, window);
            }
            XFocusIn(window) => {
                trace!("XFocusIn: {}", window);
                workspaces.focus_window(xws, &config, window);
            }
            XFocusOut(window) => {
                trace!("XFocusOut: {}", window);
                /*if workspaces.current().focused_window() == Some(window) {
                    workspaces.current_mut().unfocus_window(xws, &config);
                }*/
            }
            XButtonPress(window) => {
                workspaces.focus_window(xws, &config, window);
            }
            XKeyPress(_, mods, key) => {
                trace!("XKeyPress: {}, {}", mods, key);
                let mods = mods & !(config.mod_key | 0b10010);

                for (binding, cmd) in config.keybindings.iter() {
                    if binding.mods == mods && binding.key == key {
                        cmd.call(xws, &mut workspaces, &config)
                            .map_err(|e| error!("{}", utils::concat_error_chain(&e)))
                            .ok();
                    }
                }
            }
            _ => {}
        }

        if let Some(ref mut statusbar) = config.statusbar {
            if let Err(e) = statusbar.update(xws, &workspaces) {
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
