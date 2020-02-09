use std::io::prelude::*;
use std::path::Path;
use std::fs::{File, create_dir};

fn main() {
    let dst = Path::new(concat!(env!("HOME"), "/.xr3wm/config.rs"));
    if !dst.exists() {
        match create_dir(dst.parent().unwrap()) {
            Ok(_) => {
                let mut f = File::create(&dst).unwrap();
                f.write_all(b"#![allow(unused_imports)]
extern crate xr3wm;

use std::default::Default;
use xr3wm::core::*;

#[no_mangle]
pub extern fn configure(cfg: &mut Config) {

}").unwrap();
            }
            Err(msg) => {
                panic!("Failed to create config directory: {}", msg);
            }
        }
    }
}
