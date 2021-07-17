#![allow(dead_code)]

use x11::xlib::Window;

#[derive(Default)]
pub struct Stack {
    pub focused_window: Window,
    pub hidden: Vec<Window>,
    pub visible: Vec<Window>,
    pub urgent: Vec<Window>,
}

impl Stack {
    pub fn all(&self) -> Vec<Window> {
        self.hidden.iter().chain(self.visible.iter()).copied().collect()
    }

    pub fn len(&self) -> usize {
        self.hidden.len() + self.visible.len()
    }

    pub fn contains(&self, window: Window) -> bool {
        self.all().iter().any(|&x| x == window)
    }

    pub fn is_urgent(&self) -> bool {
        !self.urgent.is_empty()
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

    pub fn remove(&mut self, index: usize) {
        if index < self.hidden.len() {
            self.hidden.remove(index);
        } else {
            self.visible.remove(index - self.hidden.len());
        }
    }
}


