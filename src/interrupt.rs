use std::sync::atomic::{AtomicBool, Ordering};

use crate::{EXIT_INTERRUPTED, LlmfError, Result};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn handle_sigint(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

pub fn install() {
    #[cfg(unix)]
    // The handler only stores a lock-free flag; all reporting remains on the
    // normal execution path, so stderr retains its single-line contract.
    unsafe {
        libc::signal(libc::SIGINT, handle_sigint as libc::sighandler_t);
    }
}

pub fn check() -> Result<()> {
    if INTERRUPTED.load(Ordering::Relaxed) {
        Err(LlmfError::new(EXIT_INTERRUPTED, "interrupted"))
    } else {
        Ok(())
    }
}
