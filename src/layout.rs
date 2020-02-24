#![allow(dead_code)]
#![allow(clippy::new_ret_no_self)]
use std::cmp::min;
use std::fmt;
use xlib_window_system::XlibWindowSystem;
use xlib::Window;

#[derive(Clone, Copy)]
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

#[derive(Clone)]
pub enum LayoutMsg {
    Increase,
    Decrease,
    IncreaseMaster,
    DecreaseMaster,
    SplitHorizontal,
    SplitVertical,
    NextLayout,
    PrevLayout,
    FirstLayout,
    LastLayout,
    Custom(String),
}

impl fmt::Debug for LayoutMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LayoutMsg::Increase => write!(f, "Increase"),
            LayoutMsg::Decrease => write!(f, "Decrease"),
            LayoutMsg::IncreaseMaster => write!(f, "IncreasMaster"),
            LayoutMsg::DecreaseMaster => write!(f, "DecreaseMaster"),
            LayoutMsg::SplitHorizontal => write!(f, "SplitHorizontal"),
            LayoutMsg::SplitVertical => write!(f, "SplitVertical"),
            LayoutMsg::NextLayout => write!(f, "NextLayout"),
            LayoutMsg::PrevLayout => write!(f, "PrevLayout"),
            LayoutMsg::FirstLayout => write!(f, "FirstLayout"),
            LayoutMsg::LastLayout => write!(f, "LastLayout"),
            LayoutMsg::Custom(ref val) => write!(f, "Custom({})", val.clone()),
        }
    }
}

pub trait Layout {
    fn name(&self) -> String;
    fn send_msg(&mut self, LayoutMsg);
    fn apply(&self, &XlibWindowSystem, Rect, &[Window]) -> Vec<Rect>;
    fn copy<'a>(&self) -> Box<dyn Layout + 'a> {
        panic!("")
    }
}

pub struct ChooseLayout<'a> {
    layouts: Vec<Box<dyn Layout + 'a>>,
    current: usize,
}

impl<'a> ChooseLayout<'a> {
    pub fn new(layouts: Vec<Box<dyn Layout + 'a>>) -> Box<dyn Layout + 'a> {
        // add proper error handling
        if layouts.is_empty() {
            panic!("ChooseLayout needs at least one layout");
        }

        Box::new(ChooseLayout {
            layouts,
            current: 0,
        })
    }
}

impl<'a> Layout for ChooseLayout<'a> {
    fn name(&self) -> String {
        self.layouts[self.current].name()
    }

    fn send_msg(&mut self, msg: LayoutMsg) {
        let len = self.layouts.len();

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
            x => {
                self.layouts[self.current].send_msg(x);
            }
        }
    }

    fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &[Window]) -> Vec<Rect> {
        self.layouts[self.current].apply(ws, area, windows)
    }

    fn copy<'b>(&self) -> Box<dyn Layout + 'b> {
        ChooseLayout::new(self.layouts.iter().map(|x| x.copy()).collect())
    }
}

#[derive(Clone, Copy)]
pub struct TallLayout {
    num_masters: usize,
    ratio: f32,
    ratio_increment: f32,
}

impl TallLayout {
    pub fn new<'a>(num_masters: usize, ratio: f32, ratio_increment: f32) -> Box<dyn Layout + 'a> {
        Box::new(TallLayout {
            num_masters,
            ratio,
            ratio_increment,
        })
    }
}

impl Layout for TallLayout {
    fn name(&self) -> String {
        "Tall".to_string()
    }

    fn send_msg(&mut self, msg: LayoutMsg) {
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

    fn apply(&self, _: &XlibWindowSystem, area: Rect, windows: &[Window]) -> Vec<Rect> {
        (0..windows.len())
            .map(|i| {
                if i < self.num_masters {
                    let yoff = area.height / min(self.num_masters, windows.len()) as u32;

                    Rect {
                        x: area.x,
                        y: area.y + (yoff * i as u32),
                        width: (area.width as f32 *
                                (1.0 -
                                 (windows.len() > self.num_masters) as u32 as f32 *
                                 (1.0 - self.ratio)))
                            .floor() as u32,
                        height: yoff,
                    }
                } else {
                    let yoff = area.height / (windows.len() - self.num_masters) as u32;

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

    fn copy<'b>(&self) -> Box<dyn Layout + 'b> {
        Box::new(*self)
    }
}

pub struct StrutLayout<'a> {
    layout: Box<dyn Layout + 'a>,
}

impl<'a> StrutLayout<'a> {
    pub fn new(layout: Box<dyn Layout + 'a>) -> Box<dyn Layout + 'a> {
        Box::new(StrutLayout { layout: layout.copy() })
    }
}

impl<'a> Layout for StrutLayout<'a> {
    fn name(&self) -> String {
        self.layout.name()
    }

    fn send_msg(&mut self, msg: LayoutMsg) {
        self.layout.send_msg(msg);
    }

    fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &[Window]) -> Vec<Rect> {
        let mut new_area = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        let strut = ws.get_strut(area);

        new_area.x = area.x + strut.0;
        new_area.width = area.width - (strut.0 + strut.1);
        new_area.y = area.y + strut.2;
        new_area.height = area.height - (strut.2 + strut.3);

        self.layout.apply(ws, new_area, windows)
    }

    fn copy<'b>(&self) -> Box<dyn Layout + 'b> {
        StrutLayout::new(self.layout.copy())
    }
}

pub struct GapLayout<'a> {
    gap: u32,
    layout: Box<dyn Layout + 'a>,
}

impl<'a> GapLayout<'a> {
    pub fn new(gap: u32, layout: Box<dyn Layout + 'a>) -> Box<dyn Layout + 'a> {
        Box::new(GapLayout {
            gap,
            layout: layout.copy(),
        })
    }
}

impl<'a> Layout for GapLayout<'a> {
    fn name(&self) -> String {
        self.layout.name()
    }

    fn send_msg(&mut self, msg: LayoutMsg) {
        self.layout.send_msg(msg);
    }

    fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &[Window]) -> Vec<Rect> {
        let mut rects = self.layout.apply(ws, area, windows);

        for rect in rects.iter_mut() {
            rect.x += self.gap;
            rect.y += self.gap;
            rect.width -= 2 * self.gap;
            rect.height -= 2 * self.gap;
        }

        rects
    }

    fn copy<'b>(&self) -> Box<dyn Layout + 'b> {
        GapLayout::new(self.gap, self.layout.copy())
    }
}

pub struct MirrorLayout<'a> {
    layout: Box<dyn Layout + 'a>,
}

impl<'a> MirrorLayout<'a> {
    pub fn new(layout: Box<dyn Layout + 'a>) -> Box<dyn Layout + 'a> {
        Box::new(MirrorLayout { layout: layout.copy() })
    }
}

impl<'a> Layout for MirrorLayout<'a> {
    fn name(&self) -> String {
        format!("Mirror({})", self.layout.name())
    }

    fn send_msg(&mut self, msg: LayoutMsg) {
        self.layout.send_msg(msg);
    }

    fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &[Window]) -> Vec<Rect> {
        let mut rects = self.layout.apply(ws, area, windows);

        for rect in rects.iter_mut() {
            rect.x = area.width - (rect.x + rect.width);
        }

        rects
    }

    fn copy<'b>(&self) -> Box<dyn Layout + 'b> {
        MirrorLayout::new(self.layout.copy())
    }
}
