#![allow(dead_code)]

use std::cmp;
use std::iter::Iterator;
use crate::workspace::MoveOp;
use crate::xlib_window_system::XlibWindowSystem;
use crate::layout::{Layout, Rect};
use x11::xlib::Window;
use serde::{Serialize, Deserialize};
use failure::{err_msg, Error, ResultExt};

pub struct LayoutIter<'a> {
    stack: Option<&'a Stack>,
}

impl<'a> Iterator for LayoutIter<'a> {
    type Item = &'a Box<dyn Layout>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(stack) = self.stack {
            let layout = stack.layout.as_ref();

            let next_stack = stack.focus
                .and_then(|idx| match stack.nodes.get(idx) {
                    Some(Node::Stack(s)) => Some(s),
                    _ => None,
                });

            *self = LayoutIter {
                stack: next_stack,
            };

            layout
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Node {
    Window(Window),
    Stack(Stack),
}

#[derive(Serialize, Deserialize, Default)]
pub struct Stack {
    pub layout: Option<Box<dyn Layout>>,
    pub focus: Option<usize>,
    pub nodes: Vec<Node>,
    pub urgent: Vec<Window>,
}

impl Stack {
    pub fn new(layout: Option<Box<dyn Layout>>) -> Self {
        Self {
            layout,
            focus: None,
            nodes: Vec::new(),
            urgent: Vec::new(),
        }
    }

    pub fn serialize(&self) -> String {
        format!("{}:{}",
            self.all_windows().iter().map(|&x| x.to_string()).collect::<Vec<String>>().join(","),
            self.focus.unwrap_or(0))
    }

    pub fn deserialize<'a, I: Iterator<Item=&'a str>>(data_iter: &mut I, windows: &[Window]) -> Result<Self, Error> {
        let nodes = data_iter.next()
            .map(|x| {
                x.split(',')
                    .filter_map(|w| w.parse::<u64>().ok())
                    // filter obsolete windows that no longer exist
                    .filter(|w| windows.contains(w))
                    .map(Node::Window)
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        let focus = data_iter.next()
            .ok_or_else(|| err_msg("missing stack focus data"))?
            .parse::<usize>()
            .map(|x| if nodes.is_empty() {
                None
            } else {
                Some(cmp::min(nodes.len() - 1, x))
            })
            .context("failed to parse stack focus value")?;

        Ok(Self {
            focus,
            nodes,
            ..Default::default()
        })
    }

    pub fn all(&self) -> Vec<&Node> {
        self.nodes.iter().collect()
    }

    fn windows_idx(&self) -> Vec<(usize, Window)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i,x)| if let Node::Window(w) = x { Some((i, *w)) } else { None })
            .collect()
    }

    fn windows(&self) -> Vec<Window> {
        self.nodes
            .iter()
            .filter_map(|x| if let Node::Window(w) = x { Some(*w) } else { None })
            .collect()
    }

    pub fn all_windows(&self) -> Vec<Window> {
        self.windows()
            .into_iter()
            .chain(self.all_stacks().iter().map(|s| s.all_windows()).flatten())
            .collect()
    }

    pub fn all_stacks(&self) -> Vec<&Stack> {
        self.nodes
            .iter()
            .filter_map(|x| if let Node::Stack(s) = x { Some(s) } else { None })
            .collect()
    }

    pub fn all_container_mut(&mut self) -> Vec<&mut Stack> {
        self.nodes
            .iter_mut()
            .filter_map(|x| if let Node::Stack(s) = x { Some(s) } else { None })
            .collect()
    }

    pub fn layout_iter(&self) -> LayoutIter {
        LayoutIter {
            stack: Some(self)
        }
    }

    pub fn focused_window(&self) -> Option<Window> {
        match self.focused_node() {
            Some(Node::Stack(s)) => s.focused_window(),
            Some(Node::Window(w)) => Some(*w),
            _ => None,
        }
    }

    fn focused_node(&self) -> Option<&Node> {
        self.focus.and_then(move |idx| self.nodes.get(idx))
    }

    fn focused_node_mut(&mut self) -> Option<&mut Node> {
        self.focus.and_then(move |idx| self.nodes.get_mut(idx))
    }

    pub fn focus_window(&mut self, window: Window) -> bool {
        let idx = (0..self.nodes.len())
            .find(|i| match self.nodes.get_mut(*i) {
                Some(Node::Window(w)) => *w == window,
                Some(Node::Stack(s)) => s.focus_window(window),
                _ => false,
            });

        if idx.is_some() {
            self.focus = idx;
        }

        idx.is_some()
    }

    pub fn move_parent_focus(&mut self, op: MoveOp) -> Option<Window> {
        match self.focused_node_mut() {
            Some(Node::Stack(s)) => if matches!(s.focused_node(), Some(Node::Window(_))) {
                let idx = self.focus.unwrap_or(0);

                self.focus = Some(match op {
                    MoveOp::Down => (idx + 1) % self.nodes.len(),
                    MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                    MoveOp::Swap => 0,
                });
                self.focused_window()
            } else {
                s.move_parent_focus(op)
            },
            Some(Node::Window(_)) => None,
            None => None
        }
    }

    pub fn move_focus(&mut self, op: MoveOp) -> Option<Window> {
        match self.focused_node_mut() {
            Some(Node::Stack(s)) => s.move_focus(op),
            Some(Node::Window(_)) => {
                let idx = self.focus.unwrap_or(0);

                self.focus = Some(match op {
                    MoveOp::Down => (idx + 1) % self.nodes.len(),
                    MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                    MoveOp::Swap => 0,
                });
                self.focused_window()
            },
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all_windows().iter().any(|&x| x == window)
    }

    pub fn is_urgent(&self) -> bool {
        !self.urgent.is_empty() || self.all_stacks().iter().any(|s| s.is_urgent())
    }

    pub fn add_window(&mut self, window: Window) {
        match self.focused_node_mut() {
            Some(Node::Stack(s)) => s.add_window(window),
            Some(Node::Window(_)) => self.nodes.insert(self.focus.unwrap_or(0) + 1, Node::Window(window)),
            None => self.nodes.push(Node::Window(window)),
        }
    }

    pub fn add_container(&mut self, layout: Box<dyn Layout>) {
        if self.len() == 0 {
            let stack = Stack::new(Some(layout));
            self.nodes.push(Node::Stack(stack));
            self.focus = Some(0);
        } else {
            match self.focused_node_mut() {
                Some(Node::Stack(s)) => {
                    if s.len() == 1 {
                        s.layout = Some(layout)
                    } else {
                        s.add_container(layout)
                    }
                },
                Some(Node::Window(w)) => {
                    let mut stack = Stack::new(Some(layout));
                    stack.add_window(*w);
                    stack.focus = Some(0);
                    self.nodes[self.focus.unwrap_or(0)] = Node::Stack(stack)
                },
                _ => (),
            }
        }
    }

    pub fn move_window(&mut self, op: MoveOp) {
        match self.focused_node_mut() {
            Some(Node::Stack(s)) => s.move_window(op),
            _ => {
                let idx = self.focus.unwrap_or(0);

                self.focus = Some(match op {
                    MoveOp::Down => (idx + 1) % self.nodes.len(),
                    MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                    MoveOp::Swap => 0,
                });

                self.nodes.swap(idx, self.focus.expect("focused window"));
            }
        }
    }

    pub fn move_parent_window(&mut self, op: MoveOp) -> Option<Window> {
        self.focus
            .and_then(|idx| match self.nodes.get_mut(idx) {
                Some(Node::Stack(s)) => if matches!(s.focus.and_then(|idx| s.nodes.get(idx)), Some(Node::Window(_))) {
                    self.focus = Some(match op {
                        MoveOp::Down => (idx + 1) % self.nodes.len(),
                        MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                        MoveOp::Swap => 0,
                    });
                    self.focused_window()
                } else {
                    s.move_parent_focus(op)
                },
                Some(Node::Window(_)) => None,
                None => None
            })
    }

    fn find_window(&mut self, window: Window) -> Option<usize> {
        self.windows_idx().iter().find(|(_,w)| *w == window).map(|(i,_)| *i)
    }

    pub fn remove(&mut self, window: Window) -> bool {
        let res = (0..self.nodes.len())
            .find(|i| match self.nodes.get_mut(*i) {
                Some(Node::Stack(s)) => s.remove(window) && s.nodes.is_empty(),
                Some(Node::Window(w)) => *w == window,
                _ => false
            });

        if let Some(idx) = res {
            self.nodes.remove(idx);

            if self.nodes.is_empty() {
                self.focus = None;
            } else {
                self.focus = self.focus.map(|x| cmp::max(0, cmp::min(x, self.nodes.len() - 1)));
            }

            return true
        }
        false
    }

    pub fn remove_urgent(&mut self, window: Window) -> bool {
        let res = self.urgent
            .iter()
            .enumerate()
            .find(|(_,&x)| x == window)
            .map(|(i,_)| i);

        if let Some(idx) = res {
            self.urgent.swap_remove(idx);
            true
        } else {
            self.nodes
                .iter_mut()
                .filter_map(|x| if let Node::Stack(s) = x {
                    Some(s.remove_urgent(window))
                } else {
                    None
                })
                .any(|x| x)
        }
    }

    pub fn apply_layout(&self, screen: Rect, xws: &XlibWindowSystem) -> Vec<(Rect, Window)> {
        if let Some(layout) = self.layout.as_ref() {
            layout
                .apply(screen, xws, self)
                .iter()
                .enumerate()
                .filter_map(|(idx,rect)| match self.nodes.get(idx) {
                    Some(Node::Window(w)) => Some(vec![(*rect, *w)]),
                    Some(Node::Stack(s)) => Some(s.apply_layout(*rect, xws)),
                    _ => None,
                })
                .flatten()
                .collect()
        } else {
            Vec::new()
        }
    }
}
