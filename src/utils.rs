use std::process::{Command, Child};
use anyhow::{Error, Result};

pub(crate) fn xmessage(msg: &str) -> Result<Child> {
    Command::new("xmessage")
        .arg("-center")
        .arg(msg)
        .spawn()
        .map_err(|e| e.into())
}

pub(crate) fn concat_error_chain(err: &Error) -> String {
    let mut msgs = Vec::new();

    msgs.push(format!("ERROR: {}", err));

    err.chain().skip(1).for_each(|cause| msgs.push(format!("because: {}", cause)));

    msgs.as_slice().join("\n")
}
