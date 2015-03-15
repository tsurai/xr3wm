use xlib_window_system::XlibWindowSystem;
use xlib::Window;
use std::cmp::min;
use std::num::Float;
use std::fmt;

#[derive(Copy)]
pub struct Rect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32
}

impl fmt::Debug for Rect {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{{ x: {}, y: {}, width: {}, height: {} }}", self.x, self.y, self.width, self.height)
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
  Custom(String)
}

impl fmt::Debug for LayoutMsg {
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
  fn apply(&self, &XlibWindowSystem, Rect, &Vec<Window>) -> Vec<Rect>;
  fn copy<'a>(&self) -> Box<Layout + 'a> { panic!("") }
}

#[derive(Clone, Copy)]
pub struct TallLayout {
  num_masters: usize,
  ratio: f32,
  ratio_increment: f32
}

impl TallLayout {
  pub fn new<'a>(num_masters: usize, ratio: f32, ratio_increment: f32) -> Box<Layout + 'a> {
    Box::new(TallLayout {
      num_masters: num_masters,
      ratio: ratio,
      ratio_increment: ratio_increment
    })
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

  fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    (0..windows.len()).map(|i| {
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
    Box::new(self.clone())
  }
}

pub struct StrutLayout<'a> {
  layout: Box<Layout + 'a>
}

impl<'a> StrutLayout<'a> {
  pub fn new(layout: Box<Layout + 'a>) -> Box<Layout + 'a> {
    Box::new(StrutLayout {
      layout: layout.copy()
    })
  }
}

impl<'a> Layout for StrutLayout<'a> {
  fn name(&self) -> String {
    self.layout.name()
  }

  fn send_msg(&mut self, msg: LayoutMsg) {
    self.layout.send_msg(msg);
  }

  fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    let mut new_area = Rect { x: 0, y: 0, width: 0, height: 0 };
    let strut = ws.get_strut(area);

    new_area.x = area.x + strut.0;
    new_area.width = area.width - (strut.0 + strut.1);
    new_area.y = area.y + strut.2;
    new_area.height = area.height - (strut.2 + strut.3);

    self.layout.apply(ws, new_area, windows)
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    StrutLayout::new(self.layout.copy())
  }
}

pub struct GapLayout<'a> {
  gap: u32,
  layout: Box<Layout + 'a>
}

impl<'a> GapLayout<'a> {
  pub fn new(gap: u32, layout: Box<Layout + 'a>) -> Box<Layout + 'a> {
    Box::new(GapLayout {
      gap: gap,
      layout: layout.copy()
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

  fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    let mut rects = self.layout.apply(ws, area, windows);

    for rect in rects.iter_mut() {
      rect.x = rect.x + self.gap;
      rect.y = rect.y + self.gap;
      rect.width = rect.width - 2 * self.gap;
      rect.height = rect.height - 2 * self.gap;
    }

    rects
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    GapLayout::new(self.gap, self.layout.copy())
  }
}

pub struct MirrorLayout<'a> {
  layout: Box<Layout + 'a>
}

impl<'a> MirrorLayout<'a> {
  pub fn new(layout: Box<Layout + 'a>) -> Box<Layout + 'a> {
    Box::new(MirrorLayout {
      layout: layout.copy()
    })
  }
}

impl<'a> Layout for MirrorLayout<'a> {
  fn name(&self) -> String {
    format!("Mirror({})", self.layout.name())
  }

  fn send_msg(&mut self, msg: LayoutMsg) {
    self.layout.send_msg(msg);
  }

  fn apply(&self, ws: &XlibWindowSystem, area: Rect, windows: &Vec<Window>) -> Vec<Rect> {
    let mut rects = self.layout.apply(ws, area, windows);

    for rect in rects.iter_mut() {
      rect.x = area.width - (rect.x + rect.width);
    }

    rects
  }

  fn copy<'b>(&self) -> Box<Layout + 'b> {
    MirrorLayout::new(self.layout.copy())
  }
}
