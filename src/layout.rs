use xlib::Window;
use std::cmp::min;
use std::num::Float;
use std::iter::range;
use std::fmt;

#[deriving(Copy)]
pub struct Rect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32
}

impl fmt::Show for Rect {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{{ x: {}, y: {}, width: {}, height: {} }}", self.x, self.y, self.width, self.height)
  }
}

#[deriving(Clone)]
pub enum LayoutMsg {
  Increase,
  Decrease,
  IncreaseMaster,
  DecreaseMaster,
  SplitHorizontal,
  SplitVertical,
  Custom(String)
}

impl fmt::Show for LayoutMsg {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &LayoutMsg::Increase => {
        write!(f, "Increase")
      },
      &LayoutMsg::Decrease => {
        write!(f, "Decrease")
      },
      &LayoutMsg::IncreaseMaster => {
        write!(f, "IncreasMaster")
      },
      &LayoutMsg::DecreaseMaster => {
        write!(f, "DecreaseMaster")
      },
      &LayoutMsg::SplitHorizontal => {
        write!(f, "SplitHorizontal")
      },
      &LayoutMsg::SplitVertical => {
        write!(f, "SplitVertical")
      },
      &LayoutMsg::Custom(ref val) => {
        write!(f, "Custom({})", val.clone())
      }
    }
  }
}

pub trait Layout {
  fn name(&self) -> String;
  fn send_msg(&mut self, LayoutMsg);
  fn apply(&self, Rect, &Vec<Window>) -> Vec<Rect>;
  fn copy<'a>(&self) -> Box<Layout + 'a> { panic!("") }
}

#[deriving(Clone, Copy)]
pub struct TallLayout {
  num_masters: uint,
  ratio: f32,
  ratio_increment: f32
}

impl TallLayout {
  pub fn new<'a>(num_masters: uint, ratio: f32, ratio_increment: f32) -> Box<Layout + 'a> {
    box TallLayout {
      num_masters: num_masters,
      ratio: ratio,
      ratio_increment: ratio_increment
    } as Box<Layout + 'a>
  }
}

impl Layout for TallLayout {
  fn name(&self) -> String {
    String::from_str("Tall")
  }

  fn send_msg(&mut self, msg: LayoutMsg) {
    match msg {
      LayoutMsg::Increase => {
        if self.ratio + self.ratio_increment < 1.0 {
          self.ratio += self.ratio_increment;
        }
      },
      LayoutMsg::Decrease => {
        if self.ratio - self.ratio_increment > 0.0 {
          self.ratio -= self.ratio_increment;
        }
      },
      LayoutMsg::IncreaseMaster => {
        self.num_masters += 1
      },
      LayoutMsg::DecreaseMaster => {
        if self.num_masters > 1 {
          self.num_masters -= 1;
        }
      },
      _ => {}
    }
  }

  fn apply(&self, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    range(0, windows.len()).map(|i| {
      if i < self.num_masters {
        let yoff = area.height / min(self.num_masters, windows.len()) as u32;

        Rect{x: area.x, y: area.y + (yoff * i as u32), width: (area.width as f32 * (1.0 - (windows.len() > self.num_masters) as u32 as f32 * (1.0 - self.ratio))).floor() as u32 , height: yoff}
      } else {
        let yoff = area.height / (windows.len() - self.num_masters) as u32;

        Rect{x: area.x + (area.width as f32 * self.ratio).floor() as u32, y: area.y + (yoff * (i - self.num_masters) as u32), width: (area.width as f32 * (1.0 - self.ratio)).floor() as u32 , height: yoff}
      }
    }).collect()
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    box self.clone()
  }
}

pub struct BarLayout<'a> {
  top: u32,
  bottom: u32,
  layout: Box<Layout + 'a>
}

impl<'a> BarLayout<'a> {
  pub fn new(top: u32, bottom: u32, layout: Box<Layout + 'a>) -> Box<Layout + 'a> {
    box BarLayout {
      top: top,
      bottom: bottom,
      layout: layout.copy()
    } as Box<Layout + 'a>
  }
}

impl<'a> Layout for BarLayout<'a> {
  fn name(&self) -> String {
    self.layout.name()
  }

  fn send_msg(&mut self, msg: LayoutMsg) {
    self.layout.send_msg(msg);
  }

  fn apply(&self, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    self.layout.apply(Rect {x: area.x, y: area.y + self.top, width: area.width, height: area.height - (self.top + self.bottom)}, windows)
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    BarLayout::new(self.top, self.bottom, self.layout.copy())
  }
}

pub struct MirrorLayout<'a> {
  layout: Box<Layout + 'a>
}

impl<'a> MirrorLayout<'a> {
  pub fn new(layout: Box<Layout + 'a>) -> Box<Layout + 'a> {
    box MirrorLayout {
      layout: layout.copy()
    } as Box<Layout + 'a>
  }
}

impl<'a> Layout for MirrorLayout<'a> {
  fn name(&self) -> String {
    format!("Mirror({})", self.layout.name())
  }

  fn send_msg(&mut self, msg: LayoutMsg) {
    self.layout.send_msg(msg);
  }

  fn apply(&self, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    let mut rects = self.layout.apply(area, windows);

    for rect in rects.iter_mut() {
      rect.x = area.width - (rect.x + rect.width);
    }

    rects
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    MirrorLayout::new(self.layout.copy())
  }
}