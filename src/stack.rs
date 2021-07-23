#![allow(dead_code)]

use x11::xlib::Window;
use std::cmp;
use std::iter::Iterator;
use crate::workspace::MoveOp;
use crate::container::Container;
use crate::layout::Layout;
use failure::{err_msg, Error, ResultExt};

pub enum Node {
    Window(Window),
    Container(Container),
}

#[derive(Default)]
pub struct Stack {
    pub focus: Option<usize>,
    pub nodes: Vec<Node>,
    pub urgent: Vec<Window>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            focus: None,
            nodes: Vec::new(),
            urgent: Vec::new(),
        }
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
            .chain(self.all_container().iter().map(|c| c.stack.all_windows()).flatten())
            .collect()
    }

    pub fn all_container(&self) -> Vec<&Container> {
        self.nodes
            .iter()
            .filter_map(|x| if let Node::Container(c) = x { Some(c) } else { None })
            .collect()
    }

    pub fn all_container_mut(&mut self) -> Vec<&mut Container> {
        self.nodes
            .iter_mut()
            .filter_map(|x| if let Node::Container(c) = x { Some(c) } else { None })
            .collect()
    }

    pub fn focused_layouts(&self) -> Option<String> {
         self.focus
            .and_then(|idx| match self.nodes.get(idx) {
                Some(Node::Container(c)) => Some(c.layout.name() + &c.stack.focused_layouts().map(|x| format!(" / {}", x)).unwrap_or("".into())),
                Some(Node::Window(_)) => None,
                _ => None,
            })
    }

    pub fn focused_window(&self) -> Option<Window> {
        self.focus
            .and_then(|idx| match self.nodes.get(idx) {
                Some(Node::Container(c)) => c.stack.focused_window(),
                Some(Node::Window(w)) => Some(*w),
                _ => None,
            })
    }

    pub fn focus_window(&mut self, window: Window) -> bool {
        let idx = (0..self.nodes.len())
            .find(|i| match self.nodes.get_mut(*i) {
                Some(Node::Window(w)) => *w == window,
                Some(Node::Container(c)) => c.stack.focus_window(window),
                _ => false,
            });

        if idx.is_some() {
            self.focus = idx;
        }

        idx.is_some()
    }

    pub fn move_parent_focus(&mut self, op: MoveOp) -> Option<Window> {
        self.focus
            .and_then(|idx| match self.nodes.get_mut(idx) {
                Some(Node::Container(c)) => if matches!(c.stack.focus.and_then(|idx| c.stack.nodes.get(idx)), Some(Node::Window(_))) {
                    self.focus = Some(match op {
                        MoveOp::Down => (idx + 1) % self.nodes.len(),
                        MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                        MoveOp::Swap => 0,
                    });
                    self.focused_window()
                } else {
                    c.stack.move_parent_focus(op)
                },
                Some(Node::Window(_)) => None,
                None => None
            })
    }

    pub fn move_focus(&mut self, op: MoveOp) -> Option<Window> {
        if let Some(idx) = self.focus {
            if let Some(Node::Container(c)) = self.nodes.get_mut(idx) {
                c.stack.move_focus(op)
            } else {
                self.focus = Some(match op {
                    MoveOp::Down => (idx + 1) % self.nodes.len(),
                    MoveOp::Up => (idx + self.nodes.len() - 1) % self.nodes.len(),
                    MoveOp::Swap => 0,
                });
                self.focused_window()
            }
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all_windows().iter().any(|&x| x == window)
    }

    pub fn is_urgent(&self) -> bool {
        !self.urgent.is_empty() || self.all_container().iter().any(|c| c.stack.is_urgent())
    }

    pub fn add_window(&mut self, window: Window) {
        if let Some(idx) = self.focus {
            if let Some(Node::Container(c)) = self.nodes.get_mut(idx) {
                c.stack.add_window(window);
                return;
            }
            self.nodes.insert(idx + 1, Node::Window(window));
        } else {
            self.nodes.push(Node::Window(window));
        }
    }

    pub fn add_container(&mut self, layout: Box<dyn Layout>) {
        let n_nodes = self.len();

        if let Some(idx) = self.focus {
            match self.nodes.get_mut(idx) {
                Some(Node::Container(c)) => {
                    c.stack.add_container(layout)
                },
                Some(Node::Window(w)) => if n_nodes > 1 {
                    let mut container = Container::new(layout);
                    container.stack.add_window(*w);
                    container.stack.focus = Some(0);
                    self.nodes[idx] = Node::Container(container)
                }
                None => (),
            }
        }
    }

    pub fn move_window(&mut self, op: MoveOp) {
        if let Some(idx) = self.focus {
            if let Some(Node::Container(c)) = self.nodes.get_mut(idx) {
                c.stack.move_window(op);
            } else {
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
            .and_then(|idx| {
                let n_nodes = self.nodes.len();
                    match self.nodes.get_mut(idx) {
                    Some(Node::Container(c)) => if matches!(c.stack.focus.and_then(|idx| c.stack.nodes.get(idx)), Some(Node::Window(_))) {
                        if c.stack.nodes.len() == 1 {
                            self.nodes[idx] = c.stack.nodes.remove(0);
                            self.focused_window()
                        } else {
                            self.focus = Some(match op {
                                MoveOp::Down => (idx + 1) % n_nodes,
                                MoveOp::Up => (idx + n_nodes - 1) % n_nodes,
                                MoveOp::Swap => 0,
                            });

                            let window = c.stack.focused_window().expect("focused window");
                            c.stack.nodes.remove(c.stack.focus.expect("focus idx"));
                            self.nodes.insert(self.focus.expect("focus idx"), Node::Window(window));
                            Some(window)
                        }
                    } else {
                        c.stack.move_parent_focus(op)
                    },
                    Some(Node::Window(_)) => None,
                    None => None
                }
            })
    }

/*
    fn find_window(&mut self, window: Window) -> Option<usize> {
        self.windows_idx().iter().find(|(_,w)| *w == window).map(|(i,_)| *i)
    }

    pub fn index_of(&self, window: Window) -> usize {
        self.all().iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    pub fn index_of_visible(&self, window: Window) -> usize {
        self.visible.iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    pub fn index_of_hidden(&self, window: Window) -> usize {
        self.hidden.iter().enumerate().find(|&(_, &w)| w == window).map(|(i, _)| i).unwrap()
    }

    pub fn hide(&mut self, window: Window) {
        let index = self.index_of_visible(window);
        self.visible.remove(index);
        self.hidden.push(window);
    }
*/

    pub fn remove(&mut self, window: Window) -> bool {
        let res = (0..self.nodes.len())
            .find(|i| match self.nodes.get_mut(*i) {
                Some(Node::Container(c)) => c.stack.remove(window) && c.stack.nodes.is_empty(),
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
                .filter_map(|x| if let Node::Container(c) = x {
                    Some(c.stack.remove_urgent(window))
                } else {
                    None
                })
                .find(|&x| x)
                .is_some()
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
                    .map(|w| Node::Window(w))
                    .collect()
            })
            .unwrap_or(Vec::new());

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
}
