#![allow(dead_code)]
#![allow(clippy::new_ret_no_self)]

use crate::stack::{Node, Stack};
use crate::xlib_window_system::XlibWindowSystem;
use crate::ewmh;
use std::cmp::min;
use std::fmt;
use x11::xlib::Window;

#[cfg(feature = "reload")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl fmt::Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{{ x: {}, y: {}, width: {}, height: {} }}",
               self.x,
               self.y,
               self.width,
               self.height)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum LayoutMsg {
    Increase,
    Decrease,
    IncreaseMaster,
    DecreaseMaster,
    NextLayout,
    PrevLayout,
    FirstLayout,
    LastLayout,
    ResetLayout,
    NthLayout(usize),
    Custom(String),
}

impl fmt::Debug for LayoutMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LayoutMsg::Increase => write!(f, "Increase"),
            LayoutMsg::Decrease => write!(f, "Decrease"),
            LayoutMsg::IncreaseMaster => write!(f, "IncreasMaster"),
            LayoutMsg::DecreaseMaster => write!(f, "DecreaseMaster"),
            LayoutMsg::NextLayout => write!(f, "NextLayout"),
            LayoutMsg::PrevLayout => write!(f, "PrevLayout"),
            LayoutMsg::FirstLayout => write!(f, "FirstLayout"),
            LayoutMsg::NthLayout(n) => write!(f, "NthLayout: {n}"),
            LayoutMsg::LastLayout => write!(f, "LastLayout"),
            LayoutMsg::ResetLayout => write!(f, "ResetLayout"),
            LayoutMsg::Custom(ref val) => write!(f, "Custom({})", val.clone()),
        }
    }
}

#[cfg_attr(feature = "reload", typetag::serde(tag = "type"))]
pub trait Layout {
    fn name(&self) -> String;
    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg);

    fn apply(&self, area: Rect, _: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        self.simple_apply(area, &stack.nodes)
    }

    fn simple_apply(&self, _: Rect, _: &[Node]) -> Vec<Rect> {
        Vec::new()
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Choose {
    layouts: Vec<Box<dyn Layout>>,
    current: usize,
}

impl Choose {
    pub fn new(layouts: Vec<Box<dyn Layout>>) -> Box<dyn Layout> {
        // TODO: add proper error handling
        if layouts.is_empty() {
            panic!("Choose layout needs at least one layout");
        }

        Box::new(Choose {
            layouts,
            current: 0,
        })
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Choose {
    fn name(&self) -> String {
        self.layouts[self.current].name()
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        let len = self.layouts.len();
        let prev = self.current;

        match msg {
            LayoutMsg::NextLayout => {
                self.current = (self.current + 1) % len;
            }
            LayoutMsg::PrevLayout => {
                self.current = (self.current + len - 1) % len;
            }
            LayoutMsg::FirstLayout => {
                self.current = 0;
            }
            LayoutMsg::LastLayout => {
                self.current = len - 1;
            }
            LayoutMsg::NthLayout(n) => {
                if n < self.layouts.len() {
                    self.current = n
                }
            }
            x => {
                self.layouts[self.current].send_msg(xws, nodes, x);
            }
        }

        if self.current != prev {
            self.layouts[prev].send_msg(xws, nodes, LayoutMsg::ResetLayout);
        }
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        self.layouts[self.current].apply(area, xws, stack)
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Tall {
    num_masters: usize,
    ratio: f32,
    ratio_increment: f32,
}

impl Tall {
    pub fn new(num_masters: usize, ratio: f32, ratio_increment: f32) -> Box<dyn Layout> {
        Box::new(Tall {
            num_masters,
            ratio,
            ratio_increment,
        })
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Tall {
    fn name(&self) -> String {
        "Tall".to_string()
    }

    fn send_msg(&mut self, _: &XlibWindowSystem, _: &[Node], msg: LayoutMsg) {
        match msg {
            LayoutMsg::Increase => {
                if self.ratio + self.ratio_increment < 1.0 {
                    self.ratio += self.ratio_increment;
                }
            }
            LayoutMsg::Decrease => {
                if self.ratio - self.ratio_increment > self.ratio_increment {
                    self.ratio -= self.ratio_increment;
                }
            }
            LayoutMsg::IncreaseMaster => self.num_masters += 1,
            LayoutMsg::DecreaseMaster => {
                if self.num_masters > 1 {
                    self.num_masters -= 1;
                }
            }
            _ => {}
        }
    }

    fn simple_apply(&self, area: Rect, windows: &[Node]) -> Vec<Rect> {
        let nwindows = windows.len();

        (0..nwindows)
            .map(|i| {
                if i < self.num_masters {
                    let yoff = area.height / min(self.num_masters, nwindows) as u32;

                    Rect {
                        x: area.x,
                        y: area.y + (yoff * i as u32),
                        width: (area.width as f32 *
                                (1.0 -
                                 (nwindows > self.num_masters) as u32 as f32 *
                                 (1.0 - self.ratio)))
                            .floor() as u32,
                        height: yoff,
                    }
                } else {
                    let yoff = area.height / (nwindows - self.num_masters) as u32;

                    Rect {
                        x: area.x + (area.width as f32 * self.ratio).floor() as u32,
                        y: area.y + (yoff * (i - self.num_masters) as u32),
                        width: (area.width as f32 * (1.0 - self.ratio)).floor() as u32,
                        height: yoff,
                    }
                }
            })
            .collect()
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Strut {
    layout: Box<dyn Layout>,
}

impl Strut {
    pub fn new(layout: Box<dyn Layout>) -> Box<dyn Layout> {
        Box::new(Strut {
            layout
        })
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Strut {
    fn name(&self) -> String {
        self.layout.name()
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        self.layout.send_msg(xws, nodes, msg);
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        let mut new_area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        let strut = xws.compute_struts(area);

        new_area.x = area.x + strut.0;
        new_area.width = area.width - (strut.0 + strut.1);
        new_area.y = area.y + strut.2;
        new_area.height = area.height - (strut.2 + strut.3);

        self.layout.apply(new_area, xws, stack)
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Full {
    focus: Option<Window>,
    is_fullscreen: bool,
}

impl Full {
    pub fn new(is_fullscreen: bool) -> Box<dyn Layout> {
        Box::new(Full{
            focus: None,
            is_fullscreen,
        })
    }

    fn reset(xws: &XlibWindowSystem, nodes: &[Node]) {
        nodes.iter()
            .filter_map(|x| if let Node::Window(w) = x { Some(w) } else { None })
            .for_each(|&w| {
                ewmh::set_wm_state(xws, w, &[xws.get_atom("_NET_WM_STATE_FULLSCREEN")], ewmh::NET_WM_STATE_REMOVE);
            });
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Full {
    fn name(&self) -> String {
        "Full".to_string()
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        match msg {
            LayoutMsg::ResetLayout => {
                Full::reset(xws, nodes);
            },
            LayoutMsg::Custom(x) if x.as_str() == "ToggleFullscreen" => {
                self.is_fullscreen = !self.is_fullscreen;
                if !self.is_fullscreen {
                    Full::reset(xws, nodes);
                }
            },
            _ => (),
        }
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        let area = if self.is_fullscreen {
            xws.get_screen_infos()
                .into_iter()
                .find(|screen| area.x >= screen.x && area.x < screen.x + screen.width && area.y >= screen.y && area.y < screen.x + screen.width)
                .unwrap_or(area)
        } else {
            area
        };

        stack.nodes.iter()
            .enumerate()
            .map(|(i,node)| {
                match node {
                    Node::Window(w) => {
                        let is_active = ewmh::get_active_window(xws) == Some(*w);
                        let is_focused = Some(i) == stack.focus;

                        if self.is_fullscreen {
                            ewmh::set_wm_state(xws, *w, &[xws.get_atom("_NET_WM_STATE_FULLSCREEN")], (is_active && is_focused) as u64);
                        }

                        if is_active && is_focused {
                            xws.raise_window(*w);
                        }
                    },
                    Node::Stack(s) => {
                        s.all_windows().iter().for_each(|&w| {
                            if Some(i) == stack.focus {
                                xws.raise_window(w);
                            }
                        });
                    }
                }

                Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: area.height,
                }
            }).collect()
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Gap {
    screen_gap: u32,
    window_gap: u32,
    layout: Box<dyn Layout>,
}

impl Gap {
    pub fn new(screen_gap: u32, window_gap: u32, layout: Box<dyn Layout>) -> Box<dyn Layout> {
        Box::new(Gap {
            screen_gap,
            window_gap,
            layout,
        })
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Gap {
    fn name(&self) -> String {
        self.layout.name()
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        self.layout.send_msg(xws, nodes, msg);
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        let area = Rect {
            x: area.x + self.screen_gap,
            y: area.y + self.screen_gap,
            width: area.width - (2 * self.screen_gap),
            height: area.height - (2 * self.screen_gap),
        };

        let mut rects = self.layout.apply(area, xws, stack);

        for rect in rects.iter_mut() {
            rect.x += self.window_gap;
            rect.y += self.window_gap;
            rect.width -= 2 * self.window_gap;
            rect.height -= 2 * self.window_gap;
        }

        rects
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub enum MirrorStyle {
    Horizontal,
    Vertical,
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Mirror {
    style: MirrorStyle,
    layout: Box<dyn Layout>,
}

impl Mirror {
    pub fn new(style: MirrorStyle, layout: Box<dyn Layout>) -> Box<dyn Layout> {
        Box::new(Mirror {
            style,
            layout
        })
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Mirror {
    fn name(&self) -> String {
        format!("Mirror({})", self.layout.name())
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        self.layout.send_msg(xws, nodes, msg);
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        let mut rects = self.layout.apply(area, xws, stack);

        match self.style {
            MirrorStyle::Horizontal => {
                rects.iter_mut().for_each(|r| r.y = area.y + area.height - min(r.y + r.height - area.y, area.height))
            }
            MirrorStyle::Vertical => {
                rects.iter_mut().for_each(|r| r.x = area.width - min(r.x + r.width, area.width))
            }
        }

        rects
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Rotate {
    layout: Box<dyn Layout>,
}

impl Rotate {
    pub fn new(layout: Box<dyn Layout>) -> Box<dyn Layout> {
        Box::new(Rotate {
            layout
        })
    }

    fn rotate_rect(rect: Rect) -> Rect {
        Rect {
            x: rect.y,
            y: rect.x,
            width: rect.height,
            height: rect.width,
        }
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Rotate {
    fn name(&self) -> String {
        format!("Rotate({})", self.layout.name())
    }

    fn send_msg(&mut self, xws: &XlibWindowSystem, nodes: &[Node], msg: LayoutMsg) {
        self.layout.send_msg(xws, nodes, msg);
    }

    fn apply(&self, area: Rect, xws: &XlibWindowSystem, stack: &Stack) -> Vec<Rect> {
        self.layout
            .apply(Self::rotate_rect(area), xws, stack)
            .iter()
            .map(|&r| {
                Self::rotate_rect(r)
            })
            .collect()
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Horizontal;

impl Horizontal {
    pub fn new() -> Box<dyn Layout> {
        Box::new(Horizontal {})
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Horizontal {
    fn name(&self) -> String {
        "Horizontal".to_string()
    }

    fn send_msg(&mut self, _: &XlibWindowSystem, _: &[Node], _: LayoutMsg) {}

    fn simple_apply(&self, area: Rect, windows: &[Node]) -> Vec<Rect> {
        let nwindows = windows.len();

        (0..nwindows)
            .map(|i| {
                   let yoff = area.height / nwindows as u32;

                    Rect {
                        x: area.x,
                        y: area.y + (yoff * i as u32),
                        width: area.width,
                        height: yoff,
                    }
            })
            .collect()
    }
}

#[cfg_attr(feature = "reload", derive(Serialize, Deserialize))]
pub struct Vertical;

impl Vertical {
    pub fn new() -> Box<dyn Layout> {
        Box::new(Vertical {})
    }
}

#[cfg_attr(feature = "reload", typetag::serde)]
impl Layout for Vertical {
    fn name(&self) -> String {
        "Vertical".to_string()
    }

    fn send_msg(&mut self, _: &XlibWindowSystem, _: &[Node], _: LayoutMsg) {}

    fn simple_apply(&self, area: Rect, windows: &[Node]) -> Vec<Rect> {
        let nwindows = windows.len();

        (0..nwindows)
            .map(|i| {
                   let xoff = area.width / nwindows as u32;

                    Rect {
                        x: area.x + (xoff * i as u32),
                        y: area.y,
                        width: xoff,
                        height: area.height,
                    }
            })
            .collect()
    }
}
