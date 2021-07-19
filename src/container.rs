#![allow(dead_code)]

use crate::xlib_window_system::XlibWindowSystem;
use crate::stack::{Node, Stack};
use crate::layout::{Layout, LayoutMsg, Rect};
use x11::xlib::Window;
use std::fmt;

pub struct Container {
    pub stack: Stack,
    pub layout: Box<dyn Layout>,
}

impl Container {
    pub fn new(layout: Box<dyn Layout>) -> Self {
        Self {
            stack: Stack::new(),
            layout,
        }
    }

    pub fn get_layout_names(&self) -> String {
        if let Some(names) = self.stack.focused_layouts() {
            format!("{} / {}", self.layout.name(), names)
        } else {
            self.layout.name()
        }
    }

    pub fn send_layout_msg(&mut self, msg: LayoutMsg) {
        if let Some(idx) = self.stack.focus {
            match self.stack.nodes.get_mut(idx) {
                Some(Node::Container(c)) => c.send_layout_msg(msg),
                Some(Node::Window(_)) => self.layout.send_msg(msg),
                None => (),
            }
        } else {
            self.layout.send_msg(msg);
        }
    }

    pub fn apply_layout(&self, screen: Rect, xws: &XlibWindowSystem) -> Vec<(Rect, Window)> {
        self.layout
            .apply(screen, xws, &self.stack)
            .iter()
            .enumerate()
            .filter_map(|(idx,rect)| match self.stack.nodes.get(idx) {
                Some(Node::Window(w)) => Some(vec![(*rect, *w)]),
                Some(Node::Container(c)) => Some(c.apply_layout(*rect, xws)),
                _ => None,
            })
            .flatten()
            .collect()
    }
}

impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Container[{} nodes]", self.stack.len())
    }
}

