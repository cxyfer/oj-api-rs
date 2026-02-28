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

pub fn natural_sort_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut buf = String::new();
    let mut in_digit: Option<bool> = None;

    for ch in s.chars() {
        let is_digit = ch.is_ascii_digit();
        match in_digit {
            None => {
                in_digit = Some(is_digit);
                buf.push(ch);
            }
            Some(d) if d == is_digit => buf.push(ch),
            Some(d) => {
                flush_segment(&mut out, &buf, d);
                buf.clear();
                in_digit = Some(is_digit);
                buf.push(ch);
            }
        }
    }
    if let Some(d) = in_digit {
        flush_segment(&mut out, &buf, d);
    }
    out
}

fn flush_segment(out: &mut String, buf: &str, is_digit: bool) {
    const PAD: usize = 20;
    if is_digit {
        let len = buf.len();
        if len < PAD {
            for _ in 0..PAD - len {
                out.push('0');
            }
        }
        out.push_str(buf);
    } else {
        out.push_str(&buf.to_ascii_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use super::natural_sort_key;

    #[test]
    fn numeric_ordering() {
        assert!(natural_sort_key("P2000") < natural_sort_key("P10000"));
        assert!(natural_sort_key("P999") < natural_sort_key("P1000"));
    }

    #[test]
    fn pure_numeric() {
        assert!(natural_sort_key("999") < natural_sort_key("1000"));
        assert!(natural_sort_key("1") < natural_sort_key("10"));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(natural_sort_key("P1"), natural_sort_key("p1"));
        assert_eq!(natural_sort_key("ABC"), natural_sort_key("abc"));
    }

    #[test]
    fn empty_and_null_equivalent() {
        assert_eq!(natural_sort_key(""), "");
    }

    #[test]
    fn multi_segment() {
        let k = natural_sort_key("abc001_a");
        assert_eq!(k, "abc00000000000000000001_a");
    }

    #[test]
    fn pure_alpha() {
        assert!(natural_sort_key("A") < natural_sort_key("B"));
        assert!(natural_sort_key("abc") < natural_sort_key("abd"));
    }

    #[test]
    fn numeric_prefix_ordering() {
        assert!(natural_sort_key("1000A") < natural_sort_key("1000B"));
        assert!(natural_sort_key("1A") < natural_sort_key("2A"));
        assert!(natural_sort_key("9A") < natural_sort_key("10A"));
    }
}
