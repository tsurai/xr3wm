use crate::config::Config;
use crate::ewmh;
use crate::layout::{Layout, Tall};
use crate::layout::{LayoutMsg, Rect};
use crate::stack::Stack;
use crate::xlib_window_system::XlibWindowSystem;
use std::cmp;
use x11::xlib::Window;

#[cfg(feature = "reload")]
use serde::{Deserialize, Serialize};

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

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Workspace {
    pub(crate) managed: Stack,
    pub(crate) floating: Stack,
    pub(crate) tag: String,
    pub index: usize,
    pub screen: usize,
    pub visible: bool,
    pub focus: bool,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            managed: Stack::new(Some(Tall::new(1, 0.5, 0.05))),
            floating: Stack::new(None),
            tag: String::new(),
            index: 0,
            screen: 0,
            visible: false,
            focus: false,
        }
    }
}

impl Workspace {
    pub fn all(&self) -> Vec<Window> {
        self.floating
            .all_windows()
            .iter()
            .chain(self.managed.all_windows().iter())
            .copied()
            .collect()
    }

    fn all_urgent(&self) -> Vec<&Window> {
        self.floating
            .urgent
            .iter()
            .chain(self.managed.urgent.iter())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        (self.floating.len() + self.managed.len()) == 0
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

    pub fn is_floating(&self, window: Window) -> bool {
        self.floating.contains(window)
    }

    pub fn is_managed(&self, window: Window) -> bool {
        self.managed.contains(window)
    }

    pub fn is_urgent(&self) -> bool {
        self.managed.is_urgent() || self.floating.is_urgent()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn focused_window(&self) -> Option<Window> {
        self.floating
            .focused_window()
            .or_else(|| self.managed.focused_window())
    }

    pub fn add_window(&mut self, xws: &XlibWindowSystem, window: Window) {
        if !xws.is_floating_window(window) {
            debug!("Add Managed: {:#x}", window);
            self.managed.add_window(window);

            if self.floating.len() > 0 {
                debug!("Restacking");
                xws.restack_windows(self.all());
            }
        } else {
            self.floating.add_window(window);
            debug!("Add Unmanaged: {:#x}", window);
        }
    }

    pub fn nest_layout(&mut self, layout: Box<dyn Layout>) {
        if self.managed.len() > 1 {
            self.managed.add_container(layout);
        } else if let Some(s) = self.managed.all_stacks_mut().first_mut() {
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
        trace!("urgency {:#x} {}", window, urgent);

        if !urgent {
            if self.is_urgent() {
                self.remove_urgent_window(window);
            }
        } else if self.is_managed(window) {
            self.managed.urgent.push(window);
        } else {
            self.floating.urgent.push(window);
        }
    }

    fn remove_urgent_window(&mut self, window: Window) {
        if !self.managed.remove_urgent(window) {
            self.floating.remove_urgent(window);
        }
    }

    fn remove_managed(&mut self, xws: &XlibWindowSystem, window: Window) {
        self.managed.remove(window);
        xws.unmap_window(window);
    }

    fn remove_floating(&mut self, xws: &XlibWindowSystem, window: Window) {
        if !ewmh::is_window_sticky(xws, window) {
            xws.unmap_window(window);
        }

        self.floating.remove(window);
        self.floating.focus = if self.floating.nodes.is_empty() {
            None
        } else {
            Some(self.floating.nodes.len() - 1)
        };
    }

    pub fn remove_window(&mut self, xws: &XlibWindowSystem, window: Window) -> bool {
        if self.managed.contains(window) {
            trace!("Remove Managed: {:#x}", window);
            self.remove_managed(xws, window);
        } else if self.floating.contains(window) {
            trace!("Remove Unmanaged: {:#x}", window);
            self.remove_floating(xws, window);
        } else {
            return false;
        }

        true
    }

    pub fn focus_window(&mut self, xws: &XlibWindowSystem, window: Window) -> bool {
        if window == 0 || self.managed.focused_window() == Some(window) {
            return false;
        }

        if self.is_visible() {
            self.remove_urgent_window(window);
        }

        if self.floating.contains(window) {
            self.floating.focus_window(window);
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

    // TODO: impl managed and floating focus mode incl switching
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

        if let Some(window) = self
            .focused_window()
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

        xws.move_pointer(
            (rect.x + (rect.width / 2)) as i32 - 1,
            (rect.y + (rect.height / 2)) as i32,
        );
    }

    pub fn hide(&mut self, xws: &XlibWindowSystem) {
        self.visible = false;

        for &w in self
            .managed
            .all_windows()
            .iter()
            .filter(|&w| Some(*w) != self.focused_window())
        {
            xws.hide_window(w);
        }

        for &w in self
            .floating
            .all_windows()
            .iter()
            .filter(|&w| Some(*w) != self.focused_window())
        {
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
            ewmh::set_wm_desktop(xws, w, self.index);
        }

        for &w in self.floating.all_windows().iter() {
            xws.show_window(w);
            ewmh::set_wm_desktop(xws, w, self.index);
        }
    }

    pub fn redraw(&self, xws: &XlibWindowSystem, config: &Config, screens: &[Rect]) {
        trace!("Redraw workspace: {}", self.tag);
        let screen = screens[self.screen];
        let curr_focus = self.focused_window();

        for (rect, window) in self.managed.apply_layout(screen, xws) {
            let is_fullscreen = ewmh::is_window_fullscreen(xws, window);
            let border_color = if Some(window) == curr_focus {
                config.border_focus_color
            } else {
                config.border_color
            };

            if is_fullscreen {
                xws.raise_window(window);
                xws.setup_window(
                    screen.x,
                    screen.y,
                    screen.width,
                    screen.height,
                    0,
                    border_color,
                    window,
                );
            } else {
                xws.setup_window(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    config.border_width,
                    border_color,
                    window,
                );
            }
        }

        for &window in self.floating.all_windows().iter() {
            let mut rect = xws.get_geometry(window);
            rect.width = cmp::min(screen.width, rect.width + (2 * config.border_width));
            rect.height = cmp::min(screen.height, rect.height + (2 * config.border_width));
            let border_color = if Some(window) == curr_focus {
                config.border_focus_color
            } else {
                config.border_color
            };

            xws.raise_window(window);
            xws.setup_window(
                screen.x + (screen.width - rect.width) / 2,
                screen.y + (screen.height - rect.height) / 2,
                rect.width,
                rect.height,
                config.border_width,
                border_color,
                window,
            );
        }

        for &window in self.all_urgent() {
            xws.set_window_border_color(window, config.border_urgent_color);
        }

        xws.skip_enter_events();
    }
}
