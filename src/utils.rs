use std::process::{Command, Child};
use failure::*;

pub(crate) fn xmessage(msg: &str) -> Result<Child, Error> {
    Command::new("xmessage")
        .arg("-center")
        .arg(msg)
        .spawn()
        .map_err(|e| e.into())
}

pub(crate) fn concat_error_chain(err: &Error) -> String {
    let mut msgs = Vec::new();

    let mut fail: &dyn Fail = err.as_fail();
    msgs.push(format!("{}", fail));

    while let Some(cause) = fail.cause() {
        msgs.push(format!("caused by: {}", cause));

        if let Some(bt) = cause.backtrace() {
            msgs.push(format!("backtrace: {}", bt));
        }
        fail = cause;
    }

    msgs.as_slice().join("\n")
}
