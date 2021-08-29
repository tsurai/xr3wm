#![allow(unused)]

use crate::workspaces::Workspaces;
use crate::xlib_window_system::XlibWindowSystem;
use crate::config::{LogInfo, WorkspaceInfo};
use std::io::Write;
use std::process::{Command, Child, Stdio};
use failure::*;

pub struct Statusbar {
    child: Option<Child>,
    executable: String,
    args: Option<Vec<String>>,
    fn_format: Box<dyn Fn(LogInfo) -> String>,
}

impl Statusbar {
    pub fn new(executable: String,
               args: Option<Vec<String>>,
               fn_format: Box<dyn Fn(LogInfo) -> String>)
               -> Statusbar {
        Statusbar {
            child: None,
            executable,
            args,
            fn_format,
        }
    }

    pub fn xmobar() -> Statusbar {
        Statusbar::new("xmobar".to_string(),
                       None,
                       Box::new(move |info: LogInfo| -> String {
            let workspaces = info.workspaces
                .iter()
                .map(|x| {
                    let (fg, bg) = if x.current {
                        ("#00ff00", "#000000")
                    } else if x.visible {
                        ("#009900", "#000000")
                    } else if x.urgent {
                        ("#ff0000", "#000000")
                    } else {
                        ("#ffffff", "#000000")
                    };
                    format!("<fc={},{}>[{}]</fc>", fg, bg, x.tag)
                })
                .collect::<Vec<String>>()
                .join(" ");

            format!("{} | {} | {}\n",
                    workspaces,
                    info.layout_names.join("/"),
                    info.window_title)
        }))
    }

    pub fn start(&mut self) -> Result<(), Error> {
        if self.child.is_some() {
            bail!(format!("'{}' is already running", self.executable));
        }

        debug!("starting statusbar {}", self.executable);
        let mut cmd = Command::new(self.executable.clone());

        if self.args.is_some() {
            cmd.args(self.args.clone().expect("args missing").as_slice());
        }

        self.child = Some(cmd.stdin(Stdio::piped()).spawn()
            .context(format!("failed to execute '{}'", self.executable))?);

        Ok(())
    }

    pub fn update(&mut self, ws: &XlibWindowSystem, workspaces: &Workspaces) -> Result<(), Error> {
        if self.child.is_none() {
            return Ok(());
        }

        let layout_names = workspaces
            .current()
            .managed
            .layout_iter()
            .map(|x| x.name())
            .collect();

        let output = (self.fn_format)(LogInfo {
            workspaces: workspaces.all()
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    WorkspaceInfo {
                        id: i,
                        tag: x.get_tag().to_string(),
                        screen: 0,
                        current: i == workspaces.get_index(),
                        visible: x.is_visible(),
                        urgent: x.is_urgent(),
                    }
                })
                .collect(),
            layout_names,
            window_title: ws.get_window_title(workspaces.current().focused_window().unwrap_or(0)),
        });

        let stdin = self.child.as_mut()
            .ok_or_else(|| err_msg("failed to get statusbar process"))?
            .stdin
            .as_mut()
            .ok_or_else(|| err_msg("failed to get statusbar stdin"))?;

        stdin.write_all(output.as_bytes())
            .context("failed to write to statusbar stdin")?;

        Ok(())
    }
}
