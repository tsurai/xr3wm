#![allow(dead_code, unused_must_use)]

use config::Config;
use layout::{Layout, Tall};
use layout::LayoutMsg;
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use self::MoveOp::*;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::{File, remove_file};
use std::path::Path;
use std::default::Default;
use std::cmp;
use failure::*;

#[derive(Default)]
pub struct Stack {
    pub focused_window: Window,
    pub hidden: Vec<Window>,
    pub visible: Vec<Window>,
    pub urgent: Vec<Window>,
}

impl Stack {
    fn all(&self) -> Vec<Window> {
        self.hidden.iter().chain(self.visible.iter()).copied().collect()
    }

    fn len(&self) -> usize {
        self.hidden.len() + self.visible.len()
    }

    fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&x| x == window)
    }

    fn is_urgent(&self) -> bool {
        !self.urgent.is_empty()
    }

    fn index_of(&self, window: Window) -> usize {
        self.all().iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    fn index_of_visible(&self, window: Window) -> usize {
        self.visible.iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    fn index_of_hidden(&self, window: Window) -> usize {
        self.hidden.iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    fn hide(&mut self, window: Window) {
        let index = self.index_of_visible(window);
        self.visible.remove(index);
        self.hidden.push(window);
    }

    fn remove(&mut self, index: usize) {
        if index < self.hidden.len() {
            self.hidden.remove(index);
        } else {
            self.visible.remove(index - self.hidden.len());
        }
    }
}

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

pub struct Workspace {
    managed: Stack,
    unmanaged: Stack,
    tag: String,
    screen: usize,
    visible: bool,
    layout: Box<dyn Layout>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            managed: Stack::default(),
            unmanaged: Stack::default(),
            tag: String::new(),
            screen: 0,
            visible: false,
            layout: Tall::new(1, 0.5, 0.05),
        }
    }
}

pub enum MoveOp {
    Up,
    Down,
    Swap,
}

impl Workspace {
    pub fn add_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        if !ws.is_window_floating(window) {
            debug!("Add Managed: {}", window);
            self.managed.visible.push(window);

            if self.unmanaged.len() > 0 {
                debug!("Restacking");
                ws.restack_windows(self.all());
            }
        } else {
            self.unmanaged.visible.push(window);
            debug!("Add Unmanaged: {}", window);
        }

        if self.visible {
            self.redraw(ws, config);
            ws.show_window(window);
        }
    }

    pub fn new(tag: &str, layout: Box<dyn Layout>, windows: &[Window], data: &str) -> Result<Workspace, Error> {
        let data: Vec<&str> = data.split(':').collect();

        if data.len() != 8 {
            bail!("Invalid workspace data fragment count: {}", data.len());
        }

        let screen = data[0].parse::<usize>()
                .context("failed to parse screen number value")?;
        let visible = data[1].parse::<bool>()
                .context("failed to parse visible boolean value")?;

        let data: Vec<Vec<u64>> = data.iter()
            .skip(2)
            .map(|x| {
                x.split(',')
                    .filter_map(|w| w.parse::<u64>().ok())
                    .filter(|w| windows.contains(w))
                    .collect()
            })
            .collect();

        let managed = Stack {
            focused_window: *data[0].get(0).unwrap_or(&0),
            visible: data[2].clone(),
            hidden: data[3].clone(),
            ..Default::default()
        };

        let unmanaged = Stack {
            focused_window: *data[1].get(0).unwrap_or(&0),
            visible: data[4].clone(),
            hidden: data[5].clone(),
            ..Default::default()
        };

        Ok(Workspace {
            managed,
            unmanaged,
            tag: tag.to_string(),
            screen,
            visible,
            layout,
        })
    }

    pub fn serialize(&self) -> String {
        let windows = vec![
            self.managed.visible.iter().map(|&x| x.to_string()).collect::<Vec<String>>().join(","),
            self.managed.hidden.iter().map(|&x| x.to_string()).collect::<Vec<String>>().join(","),
            self.unmanaged.visible.iter().map(|&x| x.to_string()).collect::<Vec<String>>().join(","),
            self.unmanaged.hidden.iter().map(|&x| x.to_string()).collect::<Vec<String>>().join(","),
        ];

        format!("{}:{}:{}:{}:{}",
                self.screen,
                self.visible,
                self.managed.focused_window,
                self.unmanaged.focused_window,
                windows.join(":"))
    }

    fn all(&self) -> Vec<Window> {
        self.unmanaged.all().iter().chain(self.managed.all().iter()).copied().collect()
    }

    fn all_visible(&self) -> Vec<Window> {
        self.unmanaged.visible.iter().chain(self.managed.visible.iter()).copied().collect()
    }

    fn all_urgent(&self) -> Vec<Window> {
        self.unmanaged.urgent.iter().chain(self.managed.urgent.iter()).copied().collect()
    }

    pub fn get_layout(&self) -> &dyn Layout {
        &*self.layout
    }

    pub fn send_layout_message(&mut self, msg: LayoutMsg) {
        self.layout.send_msg(msg);
    }

    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }

    pub fn is_unmanaged(&self, window: Window) -> bool {
        self.unmanaged.contains(window)
    }

    pub fn is_managed(&self, window: Window) -> bool {
        self.managed.contains(window)
    }

    pub fn is_urgent(&self) -> bool {
        self.managed.is_urgent() || self.unmanaged.is_urgent()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn focused_window(&self) -> Window {
        if self.unmanaged.focused_window == 0 {
            self.managed.focused_window
        } else {
            self.unmanaged.focused_window
        }
    }

    pub fn set_urgency(&mut self, urgent: bool, ws: &XlibWindowSystem,  config: &Config, window: Window) {
        if self.all_urgent().contains(&window) {
            if !urgent {
                debug!("unset urgent {}", window);
                self.remove_urgent_window(window);
                self.redraw(ws, config);
            }
        } else if urgent {
            debug!("set urgent {}", window);
            if self.is_managed(window) {
                self.managed.urgent.push(window);
            } else {
                self.unmanaged.urgent.push(window);
            }
            self.redraw(ws, config);
        }
    }

    fn remove_urgent_window(&mut self, window: Window) {
        let index = self.all_urgent()
            .iter()
            .enumerate()
            .find(|&(_, &x)| x == window)
            .map(|(i, _)| i)
            .unwrap();
        if index < self.unmanaged.urgent.len() {
            self.unmanaged.urgent.remove(index - 1);
        } else {
            self.managed.urgent.remove(index - self.unmanaged.urgent.len());
        }
    }

    fn remove_managed(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove managed window: {}", window);
        let index = self.managed.index_of_visible(window);

        self.managed.focused_window = 0;
        ws.unmap_window(window);
        self.managed.remove(index);

        let new_focused_window = if !self.managed.visible.is_empty() {
            self.managed.visible[if index < self.managed.visible.len() {
                index
            } else {
                index - 1
            }]
        } else {
            0
        };

        if self.visible {
            self.redraw(ws, config);
            self.focus_window(ws, config, new_focused_window);
        }
    }

    fn remove_unmanaged(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove unmanaged window: {}", window);
        let index = self.unmanaged.index_of_visible(window);

        self.unmanaged.focused_window = 0;
        ws.unmap_window(window);
        self.unmanaged.remove(index);

        let new_focused_window = if !self.unmanaged.visible.is_empty() {
            self.unmanaged.visible[if index < self.unmanaged.visible.len() {
                index
            } else {
                index - 1
            }]
        } else if !self.managed.visible.is_empty() {
            self.managed.visible[self.managed.len() - 1]
        } else {
                0
        };

        if self.visible {
            self.redraw(ws, config);
            self.focus_window(ws, config, new_focused_window);
        }
    }

    pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        if self.managed.contains(window) {
            debug!("Remove Managed: {}", window);
            self.remove_managed(ws, config, window);
        } else if self.unmanaged.contains(window) {
            debug!("Remove Unmanaged: {}", window);
            self.remove_unmanaged(ws, config, window);
        }
    }

    pub fn hide_window(&mut self, window: Window) {
        if self.managed.contains(window) {
            self.managed.hide(window);
        } else {
            self.unmanaged.hide(window);
        }
    }

    pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        if window == 0 || self.unmanaged.focused_window == window || self.managed.focused_window == window {
            return;
        }

        self.unfocus_window(ws, config);

        if self.unmanaged.contains(window) {
            self.unmanaged.focused_window = window;
        } else {
            self.managed.focused_window = window;
        }

        ws.focus_window(window, config.border_focus_color);
        self.redraw(ws, config);
    }

    pub fn unfocus_window(&mut self, ws: &XlibWindowSystem, config: &Config) {
        let focused_window = self.focused_window();

        if focused_window != 0 {
            ws.set_window_border_color(focused_window, config.border_color);

            if self.unmanaged.focused_window == 0 {
                self.managed.focused_window = 0;
            } else {
                self.unmanaged.focused_window = 0;
            }
        }
    }

    pub fn move_focus(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        let windows: Vec<Window> = self.all_visible();
        let count = windows.len();

        if self.focused_window() == 0 || count < 2 {
            return;
        }

        let index = windows.iter()
            .enumerate()
            .find(|&(_, &w)| w == self.focused_window())
            .map(|(i, _)| i)
            .unwrap();

        let new_focused_window = match op {
            Up => {
                if index == 0 {
                    windows[count - 1]
                } else {
                    windows[index - 1]
                }
            }
            Down => windows[(index + 1) % count],
            Swap => windows[0],
        };

        self.focus_window(ws, config, new_focused_window);
    }

    pub fn move_window(&mut self, ws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        let focused_window = self.focused_window();
        trace!("move window: {}", focused_window);

        if focused_window == 0 || self.unmanaged.focused_window != 0 {
            return;
        }

        let pos = self.managed.index_of_visible(focused_window);
        let new_pos = match op {
            Up => {
                if pos == 0 {
                    self.managed.visible.len() - 1
                } else {
                    pos - 1
                }
            }
            Down => (pos + 1) % self.managed.visible.len(),
            Swap => {
                let master = self.managed.visible[0];
                self.managed.visible.insert(pos, master);
                self.managed.visible.remove(0);
                0
            }
        };

        self.managed.visible.remove(pos);
        self.managed.visible.insert(new_pos, focused_window);

        self.redraw(ws, config);
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&w| w == window)
    }

    pub fn unfocus(&mut self, ws: &XlibWindowSystem, config: &Config) {
        trace!("unfocus window: {}", self.focused_window());
        ws.set_window_border_color(self.focused_window(), config.border_color);
    }

    pub fn focus(&self, ws: &XlibWindowSystem, config: &Config) {
        if self.focused_window() != 0 {
            trace!("focus window: {}", self.focused_window());
            ws.focus_window(self.focused_window(), config.border_focus_color);
            ws.skip_enter_events();
        }
    }

    pub fn center_pointer(&self, ws: &XlibWindowSystem) {
        let screen = ws.get_screen_infos()[self.screen];
        ws.move_pointer((screen.x + (screen.width / 2)) as i32 - 1, (screen.y + (screen.height / 2)) as i32);
    }

    pub fn hide(&mut self, ws: &XlibWindowSystem) {
        self.visible = false;

        for &w in self.managed.visible.iter().filter(|&w| *w != self.focused_window()) {
            ws.hide_window(w);
        }

        for &w in self.unmanaged.visible.iter().filter(|&w| *w != self.focused_window()) {
            ws.hide_window(w);
        }

        ws.hide_window(self.focused_window());
    }

    pub fn show(&mut self, ws: &XlibWindowSystem, config: &Config) {
        self.visible = true;

        self.redraw(ws, config);
        for &w in self.managed.visible.iter() {
            ws.show_window(w);
        }

        for &w in self.unmanaged.visible.iter() {
            ws.show_window(w);
        }
    }

    pub fn redraw(&self, ws: &XlibWindowSystem, config: &Config) {
        trace!("Redraw...");

        let screen = ws.get_screen_infos()[self.screen];

        for (i, rect) in self.layout.apply(screen, ws, &self.managed).iter().enumerate() {
            trace!("  {}, {:?}", self.managed.visible[i], rect);
            ws.setup_window(rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                            config.border_width,
                            config.border_color,
                            self.managed.visible[i]);
        }

        for &window in self.unmanaged.visible.iter() {
            let mut rect = ws.get_geometry(window);
            rect.width = cmp::min(screen.width, rect.width + (2 * config.border_width));
            rect.height = cmp::min(screen.height, rect.height + (2 * config.border_width));

            ws.setup_window(screen.x + (screen.width - rect.width) / 2,
                            screen.y + (screen.height - rect.height) / 2,
                            rect.width,
                            rect.height,
                            config.border_width,
                            config.border_color,
                            window);
        }

        for &window in self.all_urgent().iter() {
            ws.set_window_border_color(window, config.border_urgent_color);
        }

        self.focus(ws, config);
    }
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
                if workspaces.list.iter().find(|ws| ws.screen == screen).is_none() {
                    if let Some(ws) = workspaces.list.iter_mut().filter(|ws| ws.screen == 0).nth(1) {
                        ws.screen = screen;
                    }
                }
            }

            for screen in 0..screens {
                let ws = workspaces.list.iter_mut().find(|ws| ws.screen == screen).unwrap();
                ws.visible = true;
            }

            workspaces
        }
    }

    fn load_workspaces(config: &Config, screens: usize, windows: &[Window]) -> Workspaces {
        let path = Path::new(concat!(env!("HOME"), "/.xr3wm/.tmp"));

        let mut file = BufReader::new(File::open(&path).unwrap());
        let mut cur = String::new();
        file.read_line(&mut cur);
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

    pub fn get(&self, index: usize) -> &Workspace {
        if index < self.list.len() {
            self.list.get(index).unwrap()
        } else {
            self.current()
        }
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

    pub fn get_parent_ws(&mut self, ws: &XlibWindowSystem, window: Window) -> Option<&mut Workspace> {
        ws.transient_for(window)
            .and_then(move |x| self.find_window(x))
    }

    pub fn add_window(&mut self, index: Option<usize>, ws: &XlibWindowSystem, config: &Config, window: Window) {
        if !self.contains(window) {
            let (workspace, focus) = if let Some(parent_ws) = self.get_parent_ws(ws, window) {
                (parent_ws, false)
            } else {
                (self.get_mut(index.unwrap_or_else(|| self.get_index())), true)
            };

            workspace.add_window(ws, config, window);

            if focus {
                workspace.focus_window(ws, config, window);
            }
        }
    }

    pub fn focus_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        let index = self.list
            .iter()
            .enumerate()
            .find(|&(_, workspace)| workspace.contains(window))
            .map(|(i, _)| i);

        if let Some(index) = index {
            if self.cur != index {
                self.list[index].focus_window(ws, config, window);
                self.switch_to(ws, config, index, false);
            } else {
                self.current_mut().focus_window(ws, config, window);
            }
        }
    }

    pub fn switch_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: usize, center_pointer: bool) {
        if self.cur != index && index < self.list.len() {
            // implies that the target workspace is on another screen
            if self.list[index].visible {
                if config.greedy_view {
                    self.switch_screens(index);
                    self.list[self.cur].show(ws, config);
                } else if center_pointer {
                    self.list[index].center_pointer(ws);
                }
            } else {
                self.list[index].screen = self.list[self.cur].screen;
                self.list[index].show(ws, config);
                self.list[self.cur].hide(ws);
            }

            self.list[self.cur].unfocus(ws, config);
            self.list[index].focus(ws, config);
            self.cur = index;
        }
    }

    pub fn switch_to_screen(&mut self, ws: &XlibWindowSystem, config: &Config, screen: usize) {
        if screen > self.screens - 1 {
            return;
        }

        let idx_workspace = self.list.iter()
            .enumerate()
            .filter(|&(i, workspace)| workspace.screen == screen && workspace.visible && i != self.cur)
            .map(|(i, _)| i)
            .last();

        if let Some(idx) = idx_workspace {
            self.list[self.cur].unfocus(ws, config);
            self.list[idx].focus(ws, config);
            self.list[idx].center_pointer(ws);
            self.cur = idx;
        }
    }

    pub fn move_window_to(&mut self, ws: &XlibWindowSystem, config: &Config, index: usize) {
        let window = self.list[self.cur].focused_window();
        if window == 0 || index == self.cur {
            return;
        }

        self.remove_window(ws, config, window);
        self.list[index].add_window(ws, config, window);
        self.list[index].unfocus(ws, config);
    }

    pub fn move_window_to_screen(&mut self,
                                 ws: &XlibWindowSystem,
                                 config: &Config,
                                 screen: usize) {
        if let Some((index, _)) = self.list.iter().enumerate().find(|&(_, ws)| ws.screen == screen) {
            self.move_window_to(ws, config, index);
        }
    }

    pub fn remove_window(&mut self, ws: &XlibWindowSystem, config: &Config, window: Window) {
        if let Some(workspace) = self.find_window(window) {
            workspace.remove_window(ws, config, window);
        }
    }

    pub fn hide_window(&mut self, window: Window) {
        if let Some(workspace) = self.find_window(window) {
            workspace.hide_window(window);
        }
    }

    pub fn rescreen(&mut self, ws: &XlibWindowSystem, config: &Config) {
        let new_screens = ws.get_screen_infos().len();
        let prev_screens = self.list.iter().fold(0, |acc, x| cmp::max(acc, x.screen));
        self.screens = new_screens;
        debug!("rescreen {}", new_screens);

        // move and hide workspaces if their screens got removed
        for workspace in self.list.iter_mut().filter(|ws| ws.screen > (new_screens - 1)) {
            workspace.screen = 0;
            workspace.hide(ws);
        }

        // assign the first hidden workspace to the new screen
        for screen in prev_screens + 1..new_screens {
            match self.list.iter_mut().find(|ws| !ws.visible) {
                Some(workspace) => {
                    workspace.screen = screen;
                    workspace.show(ws, config);
                }
                None => {
                    break;
                }
            }
        }

        self.list.iter_mut().find(|ws| ws.screen == 0).unwrap().show(ws, config);
    }

    pub fn find_window(&mut self, window: Window) -> Option<&mut Workspace> {
        self.list.iter_mut().find(|workspace| workspace.contains(window))
    }

    fn switch_screens(&mut self, dest: usize) {
        let screen = self.list[self.cur].screen;
        self.list[self.cur].screen = self.list[dest].screen;
        self.list[dest].screen = screen;
    }
}
