use tokio::process::{Child, Command};

/// Spawns a process in its own process group.
///
/// This allows killing the entire process tree on timeout/cancel.
#[cfg(unix)]
pub fn spawn_with_pgid(mut cmd: Command) -> std::io::Result<Child> {
    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn()
}

#[cfg(not(unix))]
pub fn spawn_with_pgid(mut cmd: Command) -> std::io::Result<Child> {
    cmd.spawn()
}

/// Kills an entire process group by sending SIGKILL to the negative PID.
///
/// Returns `true` if the signal was sent, `false` if the pid was invalid or
/// the process group no longer exists (ESRCH).
#[cfg(unix)]
pub fn kill_pgid(pid: u32) -> bool {
    if pid <= 1 {
        tracing::warn!("refusing to kill pgid {pid}: unsafe target");
        return false;
    }
    let ret = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    if ret == -1 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            tracing::debug!("pgid {pid} already exited");
        } else {
            tracing::warn!("kill_pgid({pid}) failed: {err}");
        }
        false
    } else {
        true
    }
}

#[cfg(not(unix))]
pub fn kill_pgid(_pid: u32) -> bool {
    false
}
