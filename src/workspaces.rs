#![allow(dead_code)]

use config::Config;
use layout::{Layout};
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::{File, remove_file};
use std::path::Path;
use std::default::Default;
use std::cmp;
use crate::workspace::{Workspace};

#[allow(dead_code)]
pub struct WorkspaceInfo {
    pub tags: String,
    pub layout: String,
    pub current: bool,
    pub visible: bool,
    pub urgent: bool,
}

pub struct WorkspaceConfig {
    pub tag: String,
    pub screen: usize,
    pub layout: Box<dyn Layout>,
}
pub struct Workspaces {
    list: Vec<Workspace>,
    cur: usize,
    screens: usize
}

impl Workspaces {
    pub fn new(config: &Config, screens: usize, windows: &[Window]) -> Workspaces {
        if Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp")).exists() {
            debug!("loading previous workspace state");
            Workspaces::load_workspaces(config, screens, windows)
        } else {
            let mut workspaces = Workspaces {
                list: config.workspaces
                    .iter()
                    .map(|c| {
                        Workspace {
                            tag: c.tag.clone(),
                            screen: c.screen,
                            layout: c.layout.copy(),
                            ..Default::default()
                        }
                    })
                    .collect(),
                cur: 0,
                screens
            };

            for screen in 0..screens {
                if workspaces.list.iter().find(|ws| ws.get_screen() == screen).is_none() {
                    if let Some(ws) = workspaces.list.iter_mut().filter(|ws| ws.get_screen() == 0).nth(1) {
                        ws.set_screen(screen);
                    }
                }
            }

            for screen in 0..screens {
                let ws = workspaces.list.iter_mut().find(|ws| ws.get_screen() == screen).unwrap();
                // TODO: set_visible and show are ambigious
                ws.set_visible(true);
            }

            workspaces
        }
    }

    fn load_workspaces(config: &Config, screens: usize, windows: &[Window]) -> Workspaces {
        let path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));

        let mut file = BufReader::new(File::open(&path).unwrap());
        let mut cur = String::new();
        file.read_line(&mut cur).ok();
        let lines: Vec<String> = file.lines().map(|x| x.unwrap()).collect();

        remove_file(&path).ok();

        Workspaces {
            list: config.workspaces
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    if i < lines.len() {
                        debug!("loading workspace {}", i + 1);

                        Workspace::new(&c.tag, c.layout.copy(), windows, lines.get(i).unwrap()).unwrap()
                    } else {
                        Workspace {
                            tag: c.tag.clone(),
                            screen: c.screen,
                            layout: c.layout.copy(),
                            ..Default::default()
                        }
                    }
                })
                .collect(),
            cur: cur[..cur.len() - 1].parse::<usize>().unwrap(),
            screens
        }
    }

    pub fn serialize(&self) -> String {
        format!("{}\n{}",
                self.cur,
                self.list.iter().map(|x| x.serialize()).collect::<Vec<String>>().join("\n"))
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Workspace {
        if index < self.list.len() {
            self.list.get_mut(index).unwrap()
        } else {
            self.current_mut()
        }
    }

    pub fn current(&self) -> &Workspace {
        self.list.get(self.cur).unwrap()
    }

    pub fn current_mut(&mut self) -> &mut Workspace {
        self.list.get_mut(self.cur).unwrap()
    }

    pub fn all(&self) -> &Vec<Workspace> {
        &self.list
    }

    pub fn get_index(&self) -> usize {
        self.cur
    }

    pub fn contains(&self, window: Window) -> bool {
        self.list.iter().any(|ws| ws.contains(window))
    }

    pub fn is_unmanaged(&self, window: Window) -> bool {
        self.list.iter().any(|ws| ws.is_unmanaged(window))
    }

    pub fn add_window(&mut self, index: Option<usize>, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if !self.contains(window) {
            let parent = xws.transient_for(window)
                .and_then(|x| self.find_window(x));

            let workspace = parent
                .or_else(|| index.or_else(|| Some(self.get_index())))
                .and_then(|idx| self.list.get_mut(idx))
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
                self.list.get_mut(index)
                    .expect("valid workspace")
                    .focus_window(xws, config, window);
                self.switch_to(xws, config, index, false);
            } else {
                self.current_mut().focus_window(xws, config, window);
            }
        }
    }

    pub fn switch_to(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize, center_pointer: bool) {
        if self.cur != index && index < self.list.len() {
            // implies that the target workspace is on another screen
            if self.list[index].visible {
                if config.greedy_view {
                    self.switch_screens(index);
                    self.list[self.cur].show(xws, config);
                } else if center_pointer {
                    self.list[index].center_pointer(xws);
                }
            } else {
                self.list[index].screen = self.list[self.cur].screen;
                self.list[index].show(xws, config);
                self.list[self.cur].hide(xws);
            }

            self.list[self.cur].unfocus(xws, config);
            self.list[index].focus(xws, config);
            self.cur = index;
        }
    }

    pub fn switch_to_screen(&mut self, xws: &XlibWindowSystem, config: &Config, screen: usize) {
        if screen > self.screens - 1 {
            return;
        }

        let idx_workspace = self.list.iter()
            .enumerate()
            .filter(|&(i, workspace)| workspace.screen == screen && workspace.visible && i != self.cur)
            .map(|(i, _)| i)
            .last();

        if let Some(idx) = idx_workspace {
            self.list[self.cur].unfocus(xws, config);
            self.list[idx].focus(xws, config);
            self.list[idx].center_pointer(xws);
            self.cur = idx;
        }
    }

    pub fn move_window_to(&mut self, xws: &XlibWindowSystem, config: &Config, index: usize) {
        let window = self.list[self.cur].focused_window();
        if window == 0 || index == self.cur {
            return;
        }

        self.remove_window(xws, config, window);
        self.list[index].add_window(xws, config, window);
        self.list[index].unfocus(xws, config);
    }

    pub fn move_window_to_screen(&mut self,
                                 xws: &XlibWindowSystem,
                                 config: &Config,
                                 screen: usize) {
        if let Some((index, _)) = self.list.iter().enumerate().find(|&(_, ws)| ws.screen == screen) {
            self.move_window_to(xws, config, index);
        }
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if let Some(idx) = self.find_window(window) {
            self.list.get_mut(idx)
                .expect("valid workspace")
                .remove_window(xws, config, window);
        }
    }

    pub fn hide_window(&mut self, window: Window) {
        if let Some(idx) = self.find_window(window) {
            self.list.get_mut(idx)
                .expect("valid workspace")
                .hide_window(window);
        }
    }

    pub fn rescreen(&mut self, xws: &XlibWindowSystem, config: &Config) {
        let new_screens = xws.get_screen_infos().len();
        let prev_screens = self.list.iter().fold(0, |acc, x| cmp::max(acc, x.screen));
        self.screens = new_screens;
        debug!("rescreen {}", new_screens);

        // move and hide workspaces if their screens got removed
        for workspace in self.list.iter_mut().filter(|ws| ws.screen > (new_screens - 1)) {
            workspace.screen = 0;
            workspace.hide(xws);
        }

        // assign the first hidden workspace to the new screen
        for screen in prev_screens + 1..new_screens {
            match self.list.iter_mut().find(|ws| !ws.visible) {
                Some(workspace) => {
                    workspace.screen = screen;
                    workspace.show(xws, config);
                }
                None => {
                    break;
                }
            }
        }

        self.list.iter_mut().find(|ws| ws.screen == 0).unwrap().show(xws, config);
    }

    pub fn find_window(&self, window: Window) -> Option<usize> {
        self.list.iter().enumerate().find(|(_, workspace)| workspace.contains(window)).map(|(idx, _)| idx)
    }

    fn switch_screens(&mut self, dest: usize) {
        let screen = self.list[self.cur].screen;
        self.list[self.cur].screen = self.list[dest].screen;
        self.list[dest].screen = screen;
    }
}
