use crate::config::Config;
use crate::layout::{Layout, Tall};
use crate::layout::{LayoutMsg, Rect};
use crate::stack::Stack;
use crate::xlib_window_system::XlibWindowSystem;
use crate::ewmh;
use std::cmp;
use x11::xlib::Window;
use serde::{Serialize, Deserialize};

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
    pub focus: bool,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            managed: Stack::new(Some(Tall::new(1, 0.5, 0.05))),
            unmanaged: Stack::new(None),
            tag: String::new(),
            screen: 0,
            visible: false,
            focus: false,
        }
    }
}

impl Workspace {
    pub fn all(&self) -> Vec<Window> {
        self.unmanaged.all_windows().iter().chain(self.managed.all_windows().iter()).copied().collect()
    }

    fn all_urgent(&self) -> Vec<Window> {
        self.unmanaged.urgent.iter().chain(self.managed.urgent.iter()).copied().collect()
    }

    pub fn send_layout_message(&mut self, xws: &XlibWindowSystem, msg: LayoutMsg) {
        self.managed.send_layout_msg(xws, msg);
    }

    pub fn get_tag(&self) -> &str {
        &self.tag
    }

    pub fn get_screen(&self) -> usize {
        self.screen
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
        self.unmanaged.focused_window()
            .or_else(|| self.managed.focused_window())
    }

    pub fn add_window(&mut self, xws: &XlibWindowSystem, window: Window) {
        if !xws.is_floating_window(window) {
            debug!("Add Managed: {:#x}", window);
            self.managed.add_window(window);

            if self.unmanaged.len() > 0 {
                debug!("Restacking");
                xws.restack_windows(self.all());
            }
        } else {
            self.unmanaged.add_window(window);
            debug!("Add Unmanaged: {:#x}", window);
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

    pub fn set_urgency(&mut self, urgent: bool, window: Window) {
        if !urgent {
            debug!("unset urgent {:#x}", window);
            self.remove_urgent_window(window);
        } else {
            debug!("set urgent {:#x}", window);
            if self.is_managed(window) {
                self.managed.urgent.push(window);
            } else {
                self.unmanaged.urgent.push(window);
            }
        }
    }

    fn remove_urgent_window(&mut self, window: Window) {
        if !self.managed.remove_urgent(window) {
            self.unmanaged.remove_urgent(window);
        }
    }

    fn remove_managed(&mut self, xws: &XlibWindowSystem, window: Window) {
        trace!("remove managed window: {:#x}", window);
        self.managed.remove(window);
        xws.unmap_window(window);
    }

    fn remove_unmanaged(&mut self, xws: &XlibWindowSystem, window: Window) {
        trace!("remove unmanaged window: {:#x}", window);
        xws.unmap_window(window);

        self.unmanaged.remove(window);
        self.unmanaged.focus = if self.unmanaged.nodes.is_empty() {
            None
        } else {
            Some(self.unmanaged.nodes.len() - 1)
        };
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, window: Window) -> bool {
        if self.managed.contains(window) {
            trace!("Remove Managed: {:#x}", window);
            self.remove_managed(xws, window);
        } else if self.unmanaged.contains(window) {
            trace!("Remove Unmanaged: {:#x}", window);
            self.remove_unmanaged(xws, window);
        } else {
            return false;
        }

        true
    }

    pub fn focus_window(&mut self, xws: &XlibWindowSystem, window: Window) -> bool {
        if window == 0 ||
           self.managed.focused_window() == Some(window)
        {
            return false;
        }

        if self.is_visible() {
            self.remove_urgent_window(window);
        }

        if self.unmanaged.contains(window) {
            self.unmanaged.focus_window(window);
        } else {
            self.managed.focus_window(window);
        }

        xws.focus_window(window);
        true
    }

    pub fn move_parent_focus(&mut self, op: MoveOp) -> Option<Window> {
        let prev_focus = self.focused_window();
        let new_focus = self.managed.move_parent_focus(op);

        if new_focus != prev_focus {
            new_focus
        } else {
            None
        }
    }

    // TODO: impl managed and unmanaged focus mode incl switching
    pub fn move_focus(&mut self, op: MoveOp) -> Option<Window> {
        let prev_focus = self.focused_window();
        let new_focus = self.managed.move_focus(op);

        if new_focus != prev_focus {
            new_focus
        } else {
            None
        }
    }

    pub fn move_window(&mut self, op: MoveOp) -> bool {
        if let Some(window) = self.focused_window() {
            trace!("move window: {:?}", window);

            self.managed.move_window(op);
            true
        } else {
            false
        }
    }

    pub fn move_parent_window(&mut self, op: MoveOp) -> bool {
        if let Some(window) = self.focused_window() {
            trace!("move parent window: {:?}", window);

            self.managed.move_parent_window(op);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&w| w == window)
    }

    pub fn unfocus(&mut self, xws: &XlibWindowSystem, config: &Config) {
        self.focus = false;

        if let Some(window) = self.focused_window() {
            trace!("unfocus window: {:#x}", window);
            xws.set_window_border_color(window, config.border_color);
        }
    }

    pub fn focus(&mut self, xws: &XlibWindowSystem) {
        self.focus = true;

        if let Some(window) = self.focused_window()
            .or_else(|| self.all().first().copied())
        {
            xws.focus_window(window);
        } else {
            xws.focus_window(xws.get_root_window());
        }
    }

    pub fn center_pointer(&self, xws: &XlibWindowSystem) {
        let rect = if let Some(window) = self.focused_window() {
            xws.get_geometry(window)
        } else {
            xws.get_screen_infos()[self.screen]
        };

        xws.move_pointer((rect.x + (rect.width / 2)) as i32 -1, (rect.y + (rect.height / 2)) as i32);
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

    pub fn show(&mut self, xws: &XlibWindowSystem) {
        self.visible = true;

        for &w in self.managed.all_windows().iter() {
            xws.show_window(w);
        }

        for &w in self.unmanaged.all_windows().iter() {
            xws.show_window(w);
        }
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config, screens: &[Rect]) {
        trace!("Redraw workspace: {}", self.tag);

        let screen = screens[self.screen];

        for (rect, window) in self.managed.apply_layout(screen, xws) {
            let is_fullscreen = ewmh::is_window_fullscreen(xws, window);

            if is_fullscreen {
                xws.setup_window(screen.x,
                    screen.y,
                    screen.width,
                    screen.height,
                    0,
                    config.border_color,
                    window);
            } else {
                xws.setup_window(rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    config.border_width,
                    config.border_color,
                    window);
            }
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

        if self.focus {
            if let Some(window) = self.focused_window() {
                xws.set_window_border_color(window, config.border_focus_color);
            }
        }
    }
}
