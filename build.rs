#![feature(path_ext)]

use std::io::prelude::*;
use std::path::Path;
use std::fs::{File, create_dir};

fn main() {
  let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/config.rs"));
  if !dst.exists() {
    match create_dir(dst.parent().unwrap()) {
      Ok(_) => {
        let mut f = File::create(&dst).unwrap();
        f.write_all(
b"#![allow(unused_imports)]
#![allow(alloc_jemalloc)]
extern crate alloc_jemalloc;

extern crate xr3wm;

use std::default::Default;
use xr3wm::layout::*;
use xr3wm::keycode::*;
use xr3wm::commands::*;
use xr3wm::config::*;
use xr3wm::workspaces::WorkspaceConfig;

pub extern fn configure(&mut cfg: Config) {

}").unwrap();
      },
      Err(msg) => {
        panic!("Failed to create config directory: {}", msg);
      }
    }
  }
}

