#![allow(dead_code)]

use config::Config;
use layout::{Layout, Tall};
use layout::LayoutMsg;
use xlib::Window;
use xlib_window_system::XlibWindowSystem;
use self::MoveOp::*;
use crate::stack::Stack;
use std::cmp;
use failure::*;

pub struct Workspace {
    pub(crate) managed: Stack,
    pub(crate) unmanaged: Stack,
    pub(crate) tag: String,
    pub screen: usize,
    pub visible: bool,
    pub(crate) layout: Box<dyn Layout>,
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

    pub fn get_tag(&self) -> &str {
        &self.tag
    }

    pub fn get_screen(&self) -> usize {
        self.screen
    }

    pub fn set_screen(&mut self, screen: usize) {
        self.screen = screen
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

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible
    }

    pub fn focused_window(&self) -> Window {
        if self.unmanaged.focused_window == 0 {
            self.managed.focused_window
        } else {
            self.unmanaged.focused_window
        }
    }

    pub fn add_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if !xws.is_window_floating(window) {
            debug!("Add Managed: {}", window);
            self.managed.visible.push(window);

            if self.unmanaged.len() > 0 {
                debug!("Restacking");
                xws.restack_windows(self.all());
            }
        } else {
            self.unmanaged.visible.push(window);
            debug!("Add Unmanaged: {}", window);
        }

        if self.visible {
            self.redraw(xws, config);
            xws.show_window(window);
        }
    }

    pub fn set_urgency(&mut self, urgent: bool, xws: &XlibWindowSystem,  config: &Config, window: Window) {
        if self.all_urgent().contains(&window) {
            if !urgent {
                debug!("unset urgent {}", window);
                self.remove_urgent_window(window);
                self.redraw(xws, config);
            }
        } else if urgent {
            debug!("set urgent {}", window);
            if self.is_managed(window) {
                self.managed.urgent.push(window);
            } else {
                self.unmanaged.urgent.push(window);
            }
            self.redraw(xws, config);
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

    fn remove_managed(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove managed window: {}", window);
        let index = self.managed.index_of_visible(window);

        self.managed.focused_window = 0;
        xws.unmap_window(window);
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
            self.redraw(xws, config);
            self.focus_window(xws, config, new_focused_window);
        }
    }

    fn remove_unmanaged(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove unmanaged window: {}", window);
        let index = self.unmanaged.index_of_visible(window);

        self.unmanaged.focused_window = 0;
        xws.unmap_window(window);
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
            self.redraw(xws, config);
            self.focus_window(xws, config, new_focused_window);
        }
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if self.managed.contains(window) {
            debug!("Remove Managed: {}", window);
            self.remove_managed(xws, config, window);
        } else if self.unmanaged.contains(window) {
            debug!("Remove Unmanaged: {}", window);
            self.remove_unmanaged(xws, config, window);
        }
    }

    pub fn hide_window(&mut self, window: Window) {
        if self.managed.contains(window) {
            self.managed.hide(window);
        } else {
            self.unmanaged.hide(window);
        }
    }

    pub fn focus_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if window == 0 || self.unmanaged.focused_window == window || self.managed.focused_window == window {
            return;
        }

        self.unfocus_window(xws, config);

        if self.unmanaged.contains(window) {
            self.unmanaged.focused_window = window;
        } else {
            self.managed.focused_window = window;
        }

        xws.focus_window(window, config.border_focus_color);
        self.redraw(xws, config);
    }

    pub fn unfocus_window(&mut self, xws: &XlibWindowSystem, config: &Config) {
        let focused_window = self.focused_window();

        if focused_window != 0 {
            xws.set_window_border_color(focused_window, config.border_color);

            if self.unmanaged.focused_window == 0 {
                self.managed.focused_window = 0;
            } else {
                self.unmanaged.focused_window = 0;
            }
        }
    }

    pub fn move_focus(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
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

        self.focus_window(xws, config, new_focused_window);
    }

    pub fn move_window(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
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

        self.redraw(xws, config);
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&w| w == window)
    }

    pub fn unfocus(&mut self, xws: &XlibWindowSystem, config: &Config) {
        trace!("unfocus window: {}", self.focused_window());
        xws.set_window_border_color(self.focused_window(), config.border_color);
    }

    pub fn focus(&self, xws: &XlibWindowSystem, config: &Config) {
        if self.focused_window() != 0 {
            trace!("focus window: {}", self.focused_window());
            xws.focus_window(self.focused_window(), config.border_focus_color);
            xws.skip_enter_events();
        }
    }

    pub fn center_pointer(&self, xws: &XlibWindowSystem) {
        let screen = xws.get_screen_infos()[self.screen];
        xws.move_pointer((screen.x + (screen.width / 2)) as i32 - 1, (screen.y + (screen.height / 2)) as i32);
    }

    pub fn hide(&mut self, xws: &XlibWindowSystem) {
        self.visible = false;

        for &w in self.managed.visible.iter().filter(|&w| *w != self.focused_window()) {
            xws.hide_window(w);
        }

        for &w in self.unmanaged.visible.iter().filter(|&w| *w != self.focused_window()) {
            xws.hide_window(w);
        }

        xws.hide_window(self.focused_window());
    }

    pub fn show(&mut self, xws: &XlibWindowSystem, config: &Config) {
        self.visible = true;

        self.redraw(xws, config);
        for &w in self.managed.visible.iter() {
            xws.show_window(w);
        }

        for &w in self.unmanaged.visible.iter() {
            xws.show_window(w);
        }
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config) {
        trace!("Redraw...");

        let screen = xws.get_screen_infos()[self.screen];

        for (i, rect) in self.layout.apply(screen, xws, &self.managed).iter().enumerate() {
            trace!("  {}, {:?}", self.managed.visible[i], rect);
            xws.setup_window(rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                            config.border_width,
                            config.border_color,
                            self.managed.visible[i]);
        }

        for &window in self.unmanaged.visible.iter() {
            let mut rect = xws.get_geometry(window);
            rect.width = cmp::min(screen.width, rect.width + (2 * config.border_width));
            rect.height = cmp::min(screen.height, rect.height + (2 * config.border_width));

            xws.setup_window(screen.x + (screen.width - rect.width) / 2,
                            screen.y + (screen.height - rect.height) / 2,
                            rect.width,
                            rect.height,
                            config.border_width,
                            config.border_color,
                            window);
        }

        for &window in self.all_urgent().iter() {
            xws.set_window_border_color(window, config.border_urgent_color);
        }

        self.focus(xws, config);
    }
}


