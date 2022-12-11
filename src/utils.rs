use std::process::{Command, Stdio};
use std::env;
use anyhow::{Error, Result};

#[allow(dead_code)]
pub fn xmessage(msg: &str) -> Result<()> {
    Command::new("xmessage")
        .arg("-center")
        .arg(msg)
        .output()
        .map_err(|e| e.into())
        .map(|_| ())
}

#[allow(dead_code)]
pub fn concat_error_chain(err: &Error) -> String {
    let mut msgs = Vec::new();

    msgs.push(format!("ERROR: {err}"));

    err.chain().skip(1).for_each(|cause| msgs.push(format!("because: {cause}")));

    msgs.as_slice().join("\n")
}

pub fn exec(cmd: String, args: Vec<String>) {
    if !cmd.is_empty() {
        std::thread::spawn(move || {
            match Command::new(&cmd)
                .envs(env::vars())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(&args)
                .spawn()
            {
                Ok(mut child) => {
                    child.wait().ok();
                },
                Err(e) => error!("failed to start \"{:?}\": {}", cmd, e),
            }
        });
    }
}
