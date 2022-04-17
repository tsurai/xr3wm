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
use xr3wm::layout::*;

#[no_mangle]
pub extern fn configure_workspaces() -> Vec<WorkspaceConfig> {
    (1usize..10)
        .map(|idx| {
            WorkspaceConfig {
                tag: idx.to_string(),
                screen: 0,
                layout: Strut::new(Choose::new(vec![Tall::new(1, 0.5, 0.05), Rotate::new(Tall::new(1, 0.5, 0.05)), Full::new()])),
            }
        })
        .collect()
}

#[no_mangle]
pub extern fn configure_wm() -> Config {
    let mut cfg: Config = Default::default();

    cfg
}").unwrap();
            }
            Err(msg) => {
                panic!("Failed to create config directory: {}", msg);
            }
        }
    }
}
