use crate::config::Config;
use crate::stack::Stack;
use crate::workspace::{Workspace, WorkspaceConfig};
use crate::xlib_window_system::XlibWindowSystem;
use crate::layout::Rect;
use crate::ewmh;
use std::cmp;
use std::fs::{File, remove_file};
use std::path::Path;
use std::default::Default;
use serde::{Serialize, Deserialize};
use x11::xlib::Window;
use anyhow::{Context, Result};

#[derive(Serialize, Deserialize)]
pub struct WmState {
    workspaces: Vec<Workspace>,
    struts: Vec<Window>,
    cur: usize,
    screens: Vec<Rect>
}

impl WmState {
    pub fn new(ws_cfg_list: Vec<WorkspaceConfig>, xws: &XlibWindowSystem) -> Result<WmState> {
        let state_file_path = Path::new(&Config::get_dir()?).join(".state.tmp");
        if state_file_path.exists() {
            match WmState::from_file(state_file_path) {
                Ok(state) => {
                    state.all_ws().iter().for_each(|workspace| {
                        workspace.all().iter().for_each(|&window| {
                            xws.request_window_events(window);
                        })
                    });

                    return Ok(state);
                },
                Err(e) => error!("failed to restore previous state: {}", e)
            }
        }

        debug!("creating default WmState");
        Ok(WmState {
            workspaces: ws_cfg_list
                .into_iter()
                .enumerate()
                .map(|(idx,c)| {
                    Workspace {
                        tag: c.tag.clone(),
                        screen: c.screen,
                        managed: Stack::new(Some(c.layout)),
                        focus: idx == 0,
                        ..Default::default()
                    }
                })
                .collect(),
            struts: Vec::new(),
            cur: 0,
            screens: Vec::new(),
        })
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<WmState> {
        let file = File::open(&path)
            .context("failed to open wm state serialization file")?;

        debug!("loading previous wm state");
        let ws: WmState = serde_json::from_reader(file)
            .context("failed to deserialize wm state")?;

        remove_file(path).ok();

        Ok(ws)
    }

    pub fn get_ws(&self, index: usize) -> Option<&Workspace> {
        self.workspaces.get(index)
    }

    pub fn get_ws_mut(&mut self, index: usize) -> Option<&mut Workspace> {
        self.workspaces.get_mut(index)
    }

    pub fn current_ws(&self) -> &Workspace {
        self.workspaces.get(self.cur).unwrap()
    }

    pub fn current_ws_mut(&mut self) -> &mut Workspace {
        self.workspaces.get_mut(self.cur).unwrap()
    }

    pub fn all_ws(&self) -> &Vec<Workspace> {
        &self.workspaces
    }

    pub fn all_visible_ws(&self) -> Vec<&Workspace> {
        self.workspaces
            .iter()
            .filter(|ws| ws.is_visible())
            .collect()
    }

    pub fn get_ws_index(&self) -> usize {
        self.cur
    }

    pub fn ws_count(&self) -> usize {
        self.workspaces.len()
    }

    pub fn get_screens(&self) -> &Vec<Rect> {
        &self.screens
    }

    pub fn contains(&self, window: Window) -> bool {
        self.workspaces.iter().any(|ws| ws.contains(window))
    }

    pub fn is_unmanaged(&self, window: Window) -> bool {
        self.workspaces.iter().any(|ws| ws.is_unmanaged(window))
    }

    pub fn add_window(&mut self, index: Option<usize>, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if !self.contains(window) {
            let screens = self.screens.clone();

            let parent = xws.transient_for(window)
                .and_then(|x| self.find_window(x));

            let workspace = parent
                .or_else(|| index.or_else(|| Some(self.get_ws_index())))
                .and_then(|idx| self.get_ws_mut(idx))
                .expect("valid workspace");

            workspace.add_window(xws, window);

            if parent.is_some() {
                workspace.focus_window(xws, window);
            }

            if !workspace.is_visible() {
                workspace.set_urgency(true, window);
            } else {
                workspace.redraw(xws, config, &screens);
                xws.show_window(window);
            }

            ewmh::set_client_list(xws, &self.workspaces);
        }
    }

    pub fn focus_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window, force_switch: bool) {
        if xws.get_wm_hints(window).map(|x| x.input != 0).unwrap_or(true) {
            if let Some(index) = self.find_window(window) {
                if self.cur != index {
                    let workspace = self.get_ws_mut(index)
                        .expect("valid workspace");

                    workspace.focus_window(xws, window);

                    if force_switch || workspace.is_visible() {
                        self.switch_to_ws(xws, config, index, false);
                    }
                } else {
                    self.current_ws_mut().focus_window(xws, window);
                }
            }
        }
    }

    pub fn switch_to_ws(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize, center_pointer: bool) {
        if self.cur == index || index >= self.workspaces.len() {
            return
        }

        // implies that the target workspace is on another screen
        if self.workspaces[index].visible {
            if config.greedy_view {
                self.switch_screens(xws, index);
                self.workspaces[self.cur].redraw(xws, config, &self.screens);
                self.workspaces[self.cur].show(xws);
            } else if center_pointer {
                self.workspaces[index].center_pointer(xws);
            }
        } else {
            self.workspaces[index].screen = self.workspaces[self.cur].screen;
            self.workspaces[index].redraw(xws, config, &self.screens);
            self.workspaces[index].show(xws);
            self.workspaces[self.cur].hide(xws);
        }

        self.workspaces[self.cur].unfocus(xws, config);
        self.workspaces[index].focus(xws);

        self.cur = index;

        ewmh::set_current_desktop(xws, index);
        ewmh::set_desktop_viewport(xws, self.all_ws());
    }

    pub fn switch_to_ws_at(&mut self, xws: &XlibWindowSystem, config: &Config, x: u32, y: u32, center_pointer: bool) {
        let ws_idx = self.workspaces
            .iter()
            .enumerate()
            .find(|(_,ws)| {
                let screen = self.screens[ws.get_screen()];
                x >= screen.x && x < screen.x + screen.width &&
                y >= screen.y && y < screen.y + screen.height &&
                ws.is_visible()
            })
            .map(|(idx,_)| idx);

        if let Some(idx) = ws_idx {
            self.switch_to_ws(xws, config, idx, center_pointer);
        }
    }

    pub fn switch_to_screen(&mut self, xws: &XlibWindowSystem, config: &Config, screen: usize) {
        if screen > self.screens.len() - 1 {
            return;
        }

        let idx_workspace = self.workspaces.iter()
            .enumerate()
            .filter(|&(i, workspace)| workspace.screen == screen && workspace.visible && i != self.cur)
            .map(|(i, _)| i)
            .last();

        if let Some(index) = idx_workspace {
            self.workspaces[self.cur].unfocus(xws, config);
            self.workspaces[index].focus(xws);

            self.workspaces[index].center_pointer(xws);
            self.cur = index;

            ewmh::set_current_desktop(xws, index);
        }
    }

    pub fn move_window_to_ws(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize) {
        if index == self.cur || index >= self.ws_count() {
            return;
        }

        if let Some(window) = self.current_ws().focused_window() {
            self.remove_window(xws, config, window);

            let ws = &mut self.workspaces[index];
            ws.add_window(xws, window);
            ws.focus_window(xws, window);
            ws.redraw(xws, config, &self.screens);
        }

    }

    pub fn move_window_to_screen(&mut self, xws: &XlibWindowSystem, config: &Config, screen: usize) {
        if let Some((index,_)) = self.all_ws()
            .iter()
            .enumerate()
            .find(|&(_, ws)| ws.is_visible() && ws.screen == screen)
        {
            self.move_window_to_ws(xws, config, index);
        }
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        let screens = self.screens.clone();

        if let Some(workspace) = self.get_parent_mut(window) {
            workspace.remove_window(xws, window);

            if workspace.is_visible() {
                if let Some(w) = workspace.focused_window() {
                    xws.focus_window(w);
                }
                workspace.redraw(xws, config, &screens);
            }

            ewmh::set_client_list(xws, &self.workspaces);
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
        let n_old_screens = self.screens.len();

        let screens = xws.get_screen_infos();
        trace!("xinerama screens: {:?}", screens);

        let mut new_screens: Vec<Rect> = Vec::new();

        for screen in screens {
            if let Some(old_screen) = new_screens.iter_mut()
                .find(|s| s.x == screen.x && s.y == screen.y)
            {
                    old_screen.width = cmp::min(old_screen.width, screen.width);
                    old_screen.height = cmp::min(old_screen.height, screen.height)
            } else {
                new_screens.push(screen);
            }
        }

        let n_new_screens = new_screens.len();

        // move and hide workspaces if their screens got removed
        if n_new_screens < n_old_screens {
            for workspace in self.workspaces.iter_mut().filter(|x| x.screen > n_new_screens - 1) {
                workspace.screen = 0;
                workspace.hide(xws);
            }
        } else {
            trace!("finding workspace to rescreen");
            // assign the first hidden workspace to the new screen
            for screen in n_old_screens..n_new_screens {
                trace!("rescreen for screen {}", screen);
                if let Some(workspace) = self.workspaces.iter_mut().find(|ws| !ws.is_visible()) {
                    trace!("moving workspace {}", workspace.tag);
                    workspace.screen = screen;
                    workspace.redraw(xws, config, &new_screens);
                    workspace.show(xws);
                }
            }
        }

        self.screens = new_screens;
        debug!("rescreen: {:?}", self.screens);

        ewmh::set_desktop_viewport(xws, self.all_ws());
    }

    pub fn find_window(&self, window: Window) -> Option<usize> {
        self.workspaces.iter().enumerate().find(|(_, workspace)| workspace.contains(window)).map(|(idx, _)| idx)
    }

    pub fn get_parent_mut(&mut self, window: Window) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|workspace| workspace.contains(window))
    }

    fn switch_screens(&mut self, xws: &XlibWindowSystem, dest: usize) {
        let screen = self.current_ws().get_screen();
        self.workspaces[self.cur].screen = self.workspaces[dest].screen;
        self.workspaces[dest].screen = screen;

        ewmh::set_desktop_viewport(xws, self.all_ws());
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config) {
        self.all_visible_ws()
            .iter()
            .for_each(|ws| {
                ws.redraw(xws, config, &self.screens);
            });

    }

    pub fn add_strut(&mut self, window: Window) {
        if !self.struts.contains(&window) {
            self.struts.push(window);
        }
    }

    pub fn try_remove_strut(&mut self, window: Window) -> bool {
        if let Some((idx,_)) = self.struts.iter()
            .enumerate()
            .find(|(_,&x)| x == window)
        {
            self.struts.swap_remove(idx);
            true
        } else {
            false
        }
    }
}
