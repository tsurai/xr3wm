use crate::config::Config;
use crate::stack::Stack;
use crate::workspace::{Workspace, WorkspaceConfig};
use crate::xlib_window_system::XlibWindowSystem;
use std::fs::{File, remove_file};
use std::path::Path;
use std::default::Default;
use serde::{Serialize, Deserialize};
use x11::xlib::Window;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize)]
pub struct WmState {
    workspaces: Vec<Workspace>,
    cur: usize,
    screens: usize
}

impl WmState {
    pub fn new(ws_cfg_list: Vec<WorkspaceConfig>, xws: &XlibWindowSystem) -> Result<WmState> {
        let restore_file_path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));
        if restore_file_path.exists() {
            let file = File::open(&restore_file_path)
                .context("failed to open wm state serialization file")?;

            debug!("loading previous wm state");
            let ws: WmState = serde_json::from_reader(file)
                .context("failed to deserialize wm state")?;

            remove_file(restore_file_path).ok();

            ws.all().iter().for_each(|workspace| {
                workspace.all().iter().for_each(|&window| {
                    xws.request_window_events(window);
                })
            });

            return Ok(ws);
        }

        let n_screens = xws.get_screen_infos().len();

        let mut ws = WmState {
            workspaces: ws_cfg_list
                .into_iter()
                .map(|c| {
                    Workspace {
                        tag: c.tag.clone(),
                        screen: c.screen,
                        managed: Stack::new(Some(c.layout)),
                        ..Default::default()
                    }
                })
                .collect(),
            cur: 0,
            screens: n_screens
        };

        for screen in 0..n_screens {
            if !ws.workspaces.iter().any(|ws| ws.get_screen() == screen) {
                if let Some(ws) = ws.workspaces.iter_mut().filter(|ws| ws.get_screen() == 0).nth(1) {
                    ws.set_screen(screen);
                }
            }
        }

        Ok(ws)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Workspace> {
        self.workspaces.get_mut(index)
            /*
        if index < self.workspaces.len() {
            self.workspaces.get_mut(index).unwrap()
        } else {
            self.current_mut()
        }*/
    }

    pub fn current(&self) -> &Workspace {
        self.workspaces.get(self.cur).unwrap()
    }

    pub fn current_mut(&mut self) -> &mut Workspace {
        self.workspaces.get_mut(self.cur).unwrap()
    }

    pub fn all(&self) -> &Vec<Workspace> {
        &self.workspaces
    }

    pub fn all_visible(&self) -> Vec<&Workspace> {
        self.workspaces
            .iter()
            .filter(|ws| ws.is_visible())
            .collect()
    }

    pub fn get_index(&self) -> usize {
        self.cur
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    pub fn contains(&self, window: Window) -> bool {
        self.workspaces.iter().any(|ws| ws.contains(window))
    }

    pub fn is_unmanaged(&self, window: Window) -> bool {
        self.workspaces.iter().any(|ws| ws.is_unmanaged(window))
    }

    pub fn add_window(&mut self, index: Option<usize>, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if !self.contains(window) {
            let parent = xws.transient_for(window)
                .and_then(|x| self.find_window(x));

            let workspace = parent
                .or_else(|| index.or_else(|| Some(self.get_index())))
                .and_then(|idx| self.get_mut(idx))
                .expect("valid workspace");

            workspace.add_window(xws, config, window);

            if parent.is_some() {
                workspace.focus_window(xws, config, window);
            }
        }
    }

    pub fn focus_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if let Some(index) = self.find_window(window) {
            if self.cur != index {
                self.get_mut(index)
                    .expect("valid workspace")
                    .focus_window(xws, config, window);
                self.switch_to(xws, config, index, false);
            } else {
                self.current_mut().focus_window(xws, config, window);
            }
        }
    }

    pub fn switch_to(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize, center_pointer: bool) {
        if self.cur != index && index < self.workspaces.len() {
            // implies that the target workspace is on another screen
            if self.workspaces[index].visible {
                if config.greedy_view {
                    self.switch_screens(index);
                    self.workspaces[self.cur].show(xws, config);
                } else if center_pointer {
                    self.workspaces[index].center_pointer(xws);
                }
            } else {
                self.workspaces[index].screen = self.workspaces[self.cur].screen;
                self.workspaces[index].show(xws, config);
                self.workspaces[self.cur].hide(xws);
            }

            self.workspaces[self.cur].unfocus(xws, config);
            self.workspaces[index].focus(xws, config);
            self.cur = index;
        }
    }

    pub fn switch_to_screen(&mut self, xws: &XlibWindowSystem, config: &Config, screen: usize) {
        if screen > self.screens - 1 {
            return;
        }

        let idx_workspace = self.workspaces.iter()
            .enumerate()
            .filter(|&(i, workspace)| workspace.screen == screen && workspace.visible && i != self.cur)
            .map(|(i, _)| i)
            .last();

        if let Some(idx) = idx_workspace {
            self.workspaces[self.cur].unfocus(xws, config);
            self.workspaces[idx].focus(xws, config);
            self.workspaces[idx].center_pointer(xws);
            self.cur = idx;
        }
    }

    pub fn move_window_to(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize) {
        if index == self.cur || index >= self.workspace_count() {
            return;
        }

        if let Some(window) = self.current().focused_window() {
            self.remove_window(xws, config, window);

            if let Some(workspace) = self.get_mut(index) {
                workspace.add_window(xws, config, window);
                workspace.unfocus(xws, config);
            }
        }
    }

    pub fn move_window_to_screen(&mut self,
                                 xws: &XlibWindowSystem,
                                 config: &Config,
                                 screen: usize) {
        if let Some((index, _)) = self.workspaces.iter().enumerate().find(|&(_, ws)| ws.screen == screen) {
            self.move_window_to(xws, config, index);
        }
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if let Some(idx) = self.find_window(window) {
            self.workspaces.get_mut(idx)
                .expect("valid workspace")
                .remove_window(xws, config, window);
        }
    }
/*
    pub fn hide_window(&mut self, window: Window) {
        if let Some(idx) = self.find_window(window) {
            self.list.get_mut(idx)
                .expect("valid workspace")
                .hide_window(window);
        }
    }
*/
    pub fn rescreen(&mut self, xws: &XlibWindowSystem, config: &Config) {
        let new_screens = xws.get_screen_infos().len();
        let prev_screens = self.screens;

        if new_screens == prev_screens {
            return;
        }

        self.screens = new_screens;
        debug!("rescreen {}", new_screens);

        // move and hide workspaces if their screens got removed
        if new_screens < prev_screens {
            for workspace in self.workspaces.iter_mut().filter(|x| x.screen > new_screens - 1) {
                workspace.screen = 0;
                workspace.hide(xws);
            }
        } else {
            debug!("finding workspace to rescreen");
            // assign the first hidden workspace to the new screen
            for screen in prev_screens..new_screens {
                debug!("rescreen for screen {}", screen);
                if let Some(workspace) = self.workspaces.iter_mut().find(|ws| !ws.is_visible()) {
                    debug!("moving workspace {}", workspace.tag);
                    workspace.screen = screen;
                    workspace.show(xws, config);
                }
            }
        }

        //self.workspaces.iter_mut().find(|ws| ws.screen == 0).unwrap().show(xws, config);
    }

    pub fn find_window(&self, window: Window) -> Option<usize> {
        self.workspaces.iter().enumerate().find(|(_, workspace)| workspace.contains(window)).map(|(idx, _)| idx)
    }

    pub fn get_parent_mut(&mut self, window: Window) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|workspace| workspace.contains(window))
    }

    fn switch_screens(&mut self, dest: usize) {
        let screen = self.current().get_screen();
        self.workspaces[self.cur].screen = self.workspaces[dest].screen;
        self.workspaces[dest].screen = screen;
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config) {
        self.all_visible()
            .iter()
            .for_each(|ws| ws.redraw(xws, config));
    }
}
