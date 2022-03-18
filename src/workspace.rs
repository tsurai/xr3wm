#![allow(dead_code)]

use crate::config::Config;
use crate::layout::{Layout, Tall};
use crate::layout::LayoutMsg;
use crate::stack::Stack;
use crate::xlib_window_system::XlibWindowSystem;
use std::cmp;
use x11::xlib::Window;
use serde::{Serialize, Deserialize};

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

pub enum MoveOp {
    Up,
    Down,
    Swap,
}

#[derive(Serialize, Deserialize)]
pub struct Workspace {
    pub(crate) managed: Stack,
    pub(crate) unmanaged: Stack,
    pub(crate) tag: String,
    pub screen: usize,
    pub visible: bool,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            managed: Stack::new(Some(Tall::new(1, 0.5, 0.05))),
            unmanaged: Stack::new(None),
            tag: String::new(),
            screen: 0,
            visible: false,
        }
    }
}

impl Workspace {
    pub fn deserialize(xws: &XlibWindowSystem, tag: &str, data: &str) -> Result<Workspace> {
        let mut data_iter = data.split(':');

        // get a list of all windows from the Xerver used for filterint obsolete windows
        let windows = xws.get_windows();

        let screen = data_iter.next()
            .ok_or_else(|| anyhow!("missing workspace screen data"))?
            .parse::<usize>()
            .context("failed to parse workspace screen value")?;

        let visible = data_iter.next()
            .ok_or_else(|| anyhow!("missing workspace visibility data"))?
            .parse::<bool>()
            .context("failed to parse workspace visibility value")?;

        let managed = Stack::deserialize(&mut data_iter, &windows)?;
        let unmanaged = Stack::deserialize(&mut data_iter, &windows)?;

        Ok(Workspace {
            managed,
            unmanaged,
            tag: tag.to_string(),
            screen,
            visible,
        })
    }

    pub fn all(&self) -> Vec<Window> {
        self.unmanaged.all_windows().iter().chain(self.managed.all_windows().iter()).copied().collect()
    }
/*
    fn all_visible(&self) -> Vec<Window> {
        self.unmanaged.visible.iter().chain(self.managed.stack.visible.iter()).copied().collect()
    }
*/
    fn all_urgent(&self) -> Vec<Window> {
        self.unmanaged.urgent.iter().chain(self.managed.urgent.iter()).copied().collect()
    }

    pub fn get_layout(&self) -> Option<&dyn Layout> {
        self.managed.layout.as_ref().map(|x| x.as_ref())
    }

    pub fn send_layout_message(&mut self, msg: LayoutMsg) {
        if let Some(layout) = self.managed.layout.as_mut() {
            layout.send_msg(msg);
        }
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

    pub fn focused_window(&self) -> Option<Window> {
        if self.unmanaged.focused_window().is_none() {
            self.managed.focused_window()
        } else {
            self.unmanaged.focused_window()
        }
    }

    pub fn add_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if !xws.is_window_floating(window) {
            debug!("Add Managed: {}", window);
            self.managed.add_window(window);

            if self.unmanaged.len() > 0 {
                debug!("Restacking");
                xws.restack_windows(self.all());
            }
        } else {
            self.unmanaged.add_window(window);
            debug!("Add Unmanaged: {}", window);
        }

        if self.visible {
            self.redraw(xws, config);
            xws.show_window(window);
        }
    }

    pub fn nest_layout(&mut self, layout: Box<dyn Layout>) {
        if self.managed.len() > 1 {
            self.managed.add_container(layout);
        } else  if let Some(s) = self.managed.all_container_mut().first_mut() {
            if s.len() < 1 {
                s.layout = Some(layout);
            } else {
                self.managed.add_container(layout);
            }
        } else {
            self.managed.add_container(layout);
        }
    }

    pub fn set_urgency(&mut self, urgent: bool, xws: &XlibWindowSystem,  config: &Config, window: Window) {
        if !urgent {
            debug!("unset urgent {}", window);
            self.remove_urgent_window(window);
            self.redraw(xws, config);
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
        let res = self.unmanaged.urgent
            .iter()
            .enumerate()
            .find(|&(_, &x)| x == window)
            .map(|(i, _)| i);

        if let Some(index) = res {
            if index < self.unmanaged.urgent.len() {
                self.unmanaged.urgent.remove(index - 1);
            }
        } else {
            self.managed.remove_urgent(window);
        }
    }

    fn remove_managed(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove managed window: {}", window);
        xws.unmap_window(window);
        self.managed.remove(window);

        if self.visible {
            self.redraw(xws, config);
        }
    }

    fn remove_unmanaged(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        trace!("remove unmanaged window: {}", window);
        xws.unmap_window(window);
        self.unmanaged.remove(window);
        self.unmanaged.focus = if self.unmanaged.nodes.is_empty() {
            None
        } else {
            Some(self.unmanaged.nodes.len() - 1)
        };

        if self.visible {
            self.redraw(xws, config);
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
/*
    pub fn hide_window(&mut self, window: Window) {
        if self.managed.stack.contains(window) {
            self.managed.stack.hide(window);
        } else {
            self.unmanaged.hide(window);
        }
    }
*/
    pub fn focus_window(&mut self, xws: &XlibWindowSystem, config: &Config, window: Window) {
        if window == 0 || self.unmanaged.focused_window() == Some(window) || self.managed.focused_window() == Some(window) {
            return;
        }

        self.remove_window_highlight(xws, config);

        if self.unmanaged.contains(window) {
            self.unmanaged.focus_window(window);
        } else {
            self.managed.focus_window(window);
        }

        xws.focus_window(window, config.border_focus_color);
        self.redraw(xws, config);
    }

    pub fn remove_window_highlight(&mut self, xws: &XlibWindowSystem, config: &Config) {
        if let Some(window) = self.focused_window() {
            xws.set_window_border_color(window, config.border_color);
        }
    }

    pub fn move_parent_focus(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        if self.managed.move_parent_focus(op).is_some() {
            self.redraw(xws, config);
        }
    }

    // TODO: impl managed and unmanaged focus mode incl switching
    pub fn move_focus(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        if self.managed.move_focus(op).is_some() {
            self.redraw(xws, config);
        }
    }

    pub fn move_window(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        let focused_window = self.focused_window();
        trace!("move window: {:?}", focused_window);

        if focused_window.is_none() {
            return;
        }

        self.managed.move_window(op);
        self.redraw(xws, config);
    }

    pub fn move_parent_window(&mut self, xws: &XlibWindowSystem, config: &Config, op: MoveOp) {
        let focused_window = self.focused_window();
        trace!("move parent window: {:?}", focused_window);

        if focused_window.is_none() {
            return;
        }

        self.managed.move_parent_window(op);
        self.redraw(xws, config);
    }


    pub fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&w| w == window)
    }

    pub fn unfocus(&mut self, xws: &XlibWindowSystem, config: &Config) {
        if let Some(window) = self.focused_window() {
            trace!("unfocus window: {}", window);
            xws.set_window_border_color(window, config.border_color);
        }
    }

    pub fn focus(&self, xws: &XlibWindowSystem, config: &Config) {
        if let Some(window) = self.focused_window() {
            trace!("focus window: {}", window);
            xws.focus_window(window, config.border_focus_color);
            xws.skip_enter_events();
        }
    }

    pub fn center_pointer(&self, xws: &XlibWindowSystem) {
        let screen = xws.get_screen_infos()[self.screen];
        xws.move_pointer((screen.x + (screen.width / 2)) as i32 - 1, (screen.y + (screen.height / 2)) as i32);
    }

    pub fn hide(&mut self, xws: &XlibWindowSystem) {
        self.visible = false;

        for &w in self.managed.all_windows().iter().filter(|&w| Some(*w) != self.focused_window()) {
            xws.hide_window(w);
        }

        for &w in self.unmanaged.all_windows().iter().filter(|&w| Some(*w) != self.focused_window()) {
            xws.hide_window(w);
        }

        if let Some(w) = self.focused_window() {
            xws.hide_window(w);
        }
    }

    pub fn show(&mut self, xws: &XlibWindowSystem, config: &Config) {
        self.visible = true;

        self.redraw(xws, config);
        for &w in self.managed.all_windows().iter() {
            xws.show_window(w);
        }

        for &w in self.unmanaged.all_windows().iter() {
            xws.show_window(w);
        }
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config) {
        trace!("Redraw...");

        let screen = xws.get_screen_infos()[self.screen];

        for (rect, window) in self.managed.apply_layout(screen, xws) {
            xws.setup_window(rect.x,
                rect.y,
                rect.width,
                rect.height,
                config.border_width,
                config.border_color,
                window);
        }

        for &window in self.unmanaged.all_windows().iter() {
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
