//! Login-shell environment snapshot.
//!
//! GUI/service launches (Dock, Finder, launchd, systemd) never run the user's
//! shell init, so the daemon's own PATH misses everything the shell shapes:
//! nvm's shell function, fnm multishells, asdf/mise shims, custom npm
//! prefixes, nix profiles, `~/.zshrc` exports. The hardcoded known-location
//! lists in the resolvers cover the common managers, but the only fix that
//! works for *any* setup is asking the user's actual shell: spawn it once as
//! an interactive login shell, have it print its environment between markers,
//! and keep the environment it reports. Agent children need provider keys
//! (including CPA's configured env key) as well as PATH. The snapshot stays
//! host-local and is never logged or installed into the daemon's global env.
//!
//! The snapshot is captured once per process (cached, including a negative
//! result) and is defensive about hostile shell init:
//! - `-lic` first (interactive login — nvm and friends load in rc files),
//!   falling back to `-lc` if that produces nothing (some rc files hang or
//!   `exec` a multiplexer when interactive).
//! - Output is read on a side thread into a shared buffer; the poll loop
//!   returns as soon as the end marker appears, so init that blocks *after*
//!   printing (or grandchildren inheriting the pipe) can't wedge us.
//! - A hard per-attempt timeout kills the shell.
//!
//! Set `ZERON_NO_LOGIN_SHELL=1` to disable the snapshot entirely.

use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

type Environment = Vec<(OsString, OsString)>;

static CACHE: OnceLock<Option<Environment>> = OnceLock::new();

fn login_shell_environment() -> Option<&'static Environment> {
    #[cfg(unix)]
    {
        CACHE.get_or_init(unix::capture).as_ref()
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// The PATH the user's login shell reports, captured once and cached for the
/// life of the process. `None` when disabled, non-unix, no usable shell, or
/// the shell never produced a parseable snapshot.
pub fn login_shell_path() -> Option<&'static OsStr> {
    login_shell_environment()?
        .iter()
        .find(|(key, _)| key == "PATH")
        .map(|(_, value)| value.as_os_str())
}

/// Fill gaps in a GUI/service child's inherited environment. Explicit command
/// overrides (including env_remove) and the host process's values always win.
/// Do not copy probe/shell bookkeeping into agents. PATH is composed separately.
pub(crate) fn apply_to_child(command: &mut std::process::Command) {
    if let Some(environment) = login_shell_environment() {
        apply_environment(command, environment, &|key| std::env::var_os(key));
    }
}

fn apply_environment(
    command: &mut std::process::Command,
    environment: &Environment,
    inherited: &impl Fn(&OsStr) -> Option<OsString>,
) {
    let explicit: std::collections::HashSet<OsString> =
        command.get_envs().map(|(key, _)| key.to_owned()).collect();
    for (key, value) in environment {
        if matches!(
            key.to_str(),
            Some(
                "PATH" | "PWD" | "OLDPWD" | "SHLVL" | "_" | "TERM" | "ZERON_RESOLVING_ENVIRONMENT"
            )
        ) || explicit.contains(key)
            || inherited(key).is_some()
        {
            continue;
        }
        command.env(key, value);
    }
}

/// Kick off the snapshot on a background thread so the first harness resolve
/// doesn't pay the shell-startup latency inline. Call at daemon startup.
pub fn prewarm() {
    #[cfg(unix)]
    {
        let _ = std::thread::Builder::new()
            .name("zeron-shell-env".into())
            .spawn(|| {
                let _ = login_shell_path();
            });
    }
}

#[cfg(unix)]
mod unix {
    use super::Environment;
    use std::ffi::OsString;
    use std::io::Read;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    const BEGIN_MARKER: &str = "__ZERON_SHELL_ENV_BEGIN__";
    const END_MARKER: &str = "__ZERON_SHELL_ENV_END__";
    /// Enough for any sane environment; a runaway rc file can't OOM us.
    const MAX_OUTPUT: usize = 2 * 1024 * 1024;
    const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);
    /// After the shell exits, wait this long for the pipe to flush.
    const EXIT_FLUSH_GRACE: Duration = Duration::from_millis(250);

    pub(super) fn capture() -> Option<Environment> {
        if std::env::var_os("ZERON_NO_LOGIN_SHELL").is_some_and(|v| !v.is_empty()) {
            return None;
        }
        let shell = user_shell()?;
        snapshot_environment(&shell, ATTEMPT_TIMEOUT)
    }

    /// The user's shell: `$SHELL`, then the passwd entry, then well-known
    /// defaults. Non-executables and nologin shells are skipped.
    fn user_shell() -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(s) = std::env::var_os("SHELL").filter(|s| !s.is_empty()) {
            candidates.push(PathBuf::from(s));
        }
        // systemd/launchd services often start without SHELL — passwd has it.
        if let Some(p) = passwd_shell() {
            candidates.push(p);
        }
        candidates.push(PathBuf::from("/bin/zsh"));
        candidates.push(PathBuf::from("/bin/bash"));
        candidates.push(PathBuf::from("/bin/sh"));
        candidates.into_iter().find(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string());
            let blocked = matches!(name.as_deref(), Some("nologin" | "false") | None);
            !blocked && is_executable(p)
        })
    }

    fn passwd_shell() -> Option<PathBuf> {
        // SAFETY: getpwuid's static buffer is only read here, and callers are
        // serialized through the OnceLock init above.
        unsafe {
            let pw = libc::getpwuid(libc::getuid());
            if pw.is_null() || (*pw).pw_shell.is_null() {
                return None;
            }
            let shell = std::ffi::CStr::from_ptr((*pw).pw_shell);
            (!shell.to_bytes().is_empty())
                .then(|| PathBuf::from(std::ffi::OsStr::from_bytes(shell.to_bytes())))
        }
    }

    fn is_executable(p: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }

    /// Flag sets to try, most-loaded first. csh/tcsh reject `-l` combined with
    /// `-c`; fish runs config.fish for every invocation, so `-l` alone loads
    /// everything without interactive-mode side effects.
    fn attempt_flag_sets(shell: &Path) -> Vec<Vec<&'static str>> {
        let name = shell
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        match name.as_str() {
            "csh" | "tcsh" => vec![vec!["-c"]],
            "fish" => vec![vec!["-l", "-c"], vec!["-c"]],
            _ => vec![vec!["-l", "-i", "-c"], vec!["-l", "-c"]],
        }
    }

    /// NUL-delimited records preserve multiline/non-UTF8 secrets and cannot
    /// mistake a newline in a value for a second environment variable. Use the
    /// absolute env path so unusual shell PATHs cannot break or shadow the probe.
    fn snapshot_environment(shell: &Path, timeout: Duration) -> Option<Environment> {
        let script = format!("echo {BEGIN_MARKER}; /usr/bin/env -0; echo {END_MARKER}");
        for flags in attempt_flag_sets(shell) {
            let output = run_and_capture(shell, &flags, &script, timeout);
            if let Some(environment) = parse_snapshot_environment(&output) {
                return Some(environment);
            }
        }
        None
    }

    /// Spawn the shell and collect stdout until the end marker appears, the
    /// child exits (plus a flush grace), or the timeout kills it. The reader
    /// lives on its own thread appending into a shared buffer, so a shell that
    /// blocks after printing — or a grandchild that inherits the pipe and
    /// never closes it — can't hang us on EOF.
    fn run_and_capture(shell: &Path, flags: &[&str], script: &str, timeout: Duration) -> Vec<u8> {
        let mut cmd = std::process::Command::new(shell);
        cmd.args(flags)
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            // Let rc files detect (and skip work for) this probe, mirroring
            // VSCODE_RESOLVING_ENVIRONMENT; TERM=dumb quiets fancy prompts.
            .env("ZERON_RESOLVING_ENVIRONMENT", "1")
            .env("TERM", "dumb");
        let Ok(mut child) = cmd.spawn() else {
            return Vec::new();
        };
        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        if let Some(mut stdout) = child.stdout.take() {
            let buf = Arc::clone(&buf);
            let _ = std::thread::Builder::new()
                .name("zeron-shell-env-read".into())
                .spawn(move || {
                    let mut chunk = [0u8; 8192];
                    loop {
                        match stdout.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                let mut b = buf
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                                if b.len() >= MAX_OUTPUT {
                                    break;
                                }
                                b.extend_from_slice(&chunk[..n]);
                            }
                        }
                    }
                });
        }
        let deadline = Instant::now() + timeout;
        let mut exited_at: Option<Instant> = None;
        let mut scanned = 0usize;
        loop {
            {
                let b = buf
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                // Only scan the unscanned tail (minus marker-length overlap).
                let from = scanned.saturating_sub(END_MARKER.len());
                if find_subslice(&b[from..], END_MARKER.as_bytes()).is_some() {
                    break;
                }
                scanned = b.len();
            }
            match exited_at {
                Some(at) if at.elapsed() >= EXIT_FLUSH_GRACE => break,
                Some(_) => {}
                None => match child.try_wait() {
                    Ok(Some(_)) => exited_at = Some(Instant::now()),
                    Ok(None) if Instant::now() >= deadline => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    Ok(None) => {}
                    Err(_) => break,
                },
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if exited_at.is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let b = buf
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        b.clone()
    }

    /// Extract environment from the `env` dump between the LAST begin marker and the
    /// first end marker after it (rc noise printed before our command — or a
    /// marker echoed by init itself — lands before the real one).
    fn parse_snapshot_environment(output: &[u8]) -> Option<Environment> {
        let begin = rfind_subslice(output, BEGIN_MARKER.as_bytes())?;
        let after = &output[begin + BEGIN_MARKER.len()..];
        let end = find_subslice(after, END_MARKER.as_bytes())?;
        let records = after[..end]
            .strip_prefix(b"\r\n")
            .or_else(|| after[..end].strip_prefix(b"\n"))?;
        if !records.ends_with(b"\0") {
            return None;
        }
        let mut environment = Vec::new();
        for record in records.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let separator = record.iter().position(|b| *b == b'=')?;
            let (key, value) = (&record[..separator], &record[separator + 1..]);
            if key.is_empty()
                || !(key[0].is_ascii_alphabetic() || key[0] == b'_')
                || !key.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                continue;
            }
            environment.push((
                OsString::from_vec(key.to_vec()),
                OsString::from_vec(value.to_vec()),
            ));
        }
        environment
            .iter()
            .any(|(key, value)| key == "PATH" && !value.is_empty())
            .then_some(environment)
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .rposition(|window| window == needle)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::ffi::OsStr;
        use std::os::unix::fs::PermissionsExt;

        fn fake_shell(dir: &Path, body: &str) -> PathBuf {
            let path = dir.join("fake-shell");
            std::fs::write(&path, body).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        /// A fake $SHELL skeleton: consume flags, exec the `-c` payload.
        const RUN_PAYLOAD: &str = r#"
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-c" ]; then shift; exec /bin/sh -c "$1"; fi
  shift
done
exit 1
"#;

        fn snapshot_path(shell: &Path, timeout: Duration) -> Option<OsString> {
            snapshot_environment(shell, timeout)?
                .into_iter()
                .find(|(key, _)| key == "PATH")
                .map(|(_, value)| value)
        }

        fn parse_snapshot_path(output: &[u8]) -> Option<OsString> {
            parse_snapshot_environment(output)?
                .into_iter()
                .find(|(key, _)| key == "PATH")
                .map(|(_, value)| value)
        }

        #[test]
        fn parses_path_between_markers() {
            let output = format!(
                "rc noise\n{BEGIN_MARKER}\nHOME=/home/u\0PATH=/custom/bin:/usr/bin\0X=y\0{END_MARKER}\ntrailing"
            );
            let path = parse_snapshot_path(output.as_bytes()).unwrap();
            assert_eq!(path, OsString::from("/custom/bin:/usr/bin"));
        }

        #[test]
        fn ignores_marker_echoed_by_init() {
            // rc noise that happens to contain the begin marker but no PATH
            // after it must not shadow the real snapshot.
            let output =
                format!("{BEGIN_MARKER}\ngarbage\n{BEGIN_MARKER}\nPATH=/real/bin\0{END_MARKER}\n");
            let path = parse_snapshot_path(output.as_bytes()).unwrap();
            assert_eq!(path, OsString::from("/real/bin"));
        }

        #[test]
        fn preserves_secret_values_without_parsing_newlines_as_variables() {
            let output = format!(
                "{BEGIN_MARKER}\nPATH=/bin\0CPA_API_KEY=first=second\nINJECTED=not-a-variable\r\n\0EMPTY=\0BINARY="
            );
            let mut output = output.into_bytes();
            output.extend_from_slice(b"\xff\0");
            output.extend_from_slice(END_MARKER.as_bytes());
            let environment = parse_snapshot_environment(&output).unwrap();
            assert_eq!(
                environment,
                vec![
                    ("PATH".into(), "/bin".into()),
                    (
                        "CPA_API_KEY".into(),
                        "first=second\nINJECTED=not-a-variable\r\n".into()
                    ),
                    ("EMPTY".into(), "".into()),
                    ("BINARY".into(), OsString::from_vec(vec![0xff])),
                ]
            );
        }

        #[test]
        fn rejects_incomplete_or_pathless_snapshots() {
            for records in ["PATH=/bin", "CPA_API_KEY=test\0", "PATH=\0"] {
                let output = format!("{BEGIN_MARKER}\n{records}{END_MARKER}\n");
                assert!(parse_snapshot_environment(output.as_bytes()).is_none());
            }
        }

        #[test]
        fn child_environment_preserves_overrides_removals_and_host_values() {
            let environment = vec![
                ("CPA_API_KEY".into(), "shell-key".into()),
                ("PROVIDER_ENDPOINT".into(), "shell-endpoint".into()),
                ("REMOVED_KEY".into(), "must-not-return".into()),
                ("HOST_KEY".into(), "must-not-override".into()),
                ("PWD".into(), "/probe".into()),
                ("TERM".into(), "dumb".into()),
                ("ZERON_RESOLVING_ENVIRONMENT".into(), "1".into()),
            ];
            let mut command = std::process::Command::new("unused");
            command.env("PROVIDER_ENDPOINT", "explicit-endpoint");
            command.env_remove("REMOVED_KEY");
            super::super::apply_environment(&mut command, &environment, &|key| {
                (key == "HOST_KEY").then(|| "host-value".into())
            });
            let values: std::collections::HashMap<_, _> = command.get_envs().collect();
            assert_eq!(
                values[OsStr::new("CPA_API_KEY")],
                Some(OsStr::new("shell-key"))
            );
            assert_eq!(
                values[OsStr::new("PROVIDER_ENDPOINT")],
                Some(OsStr::new("explicit-endpoint"))
            );
            assert_eq!(values[OsStr::new("REMOVED_KEY")], None);
            for key in ["HOST_KEY", "PWD", "TERM", "ZERON_RESOLVING_ENVIRONMENT"] {
                assert!(!values.contains_key(OsStr::new(key)));
            }
        }

        #[test]
        fn snapshots_path_from_fake_shell() {
            let dir = tempfile::tempdir().unwrap();
            let shell = fake_shell(
                dir.path(),
                &format!(
                    "#!/bin/sh\nPATH=\"/zeron-test/custom/bin:/usr/bin:/bin\"; export PATH\n{RUN_PAYLOAD}"
                ),
            );
            let path = snapshot_path(&shell, Duration::from_secs(10)).unwrap();
            let path = path.to_string_lossy();
            assert!(path.starts_with("/zeron-test/custom/bin:"), "got: {path}");
        }

        #[test]
        fn falls_back_when_interactive_attempt_hangs() {
            let dir = tempfile::tempdir().unwrap();
            // Simulates rc files that wedge only in interactive mode (`exec
            // tmux` and friends): sleep forever when -i is present.
            let shell = fake_shell(
                dir.path(),
                &format!(
                    "#!/bin/sh\ncase \" $* \" in *\" -i \"*) sleep 60;; esac\nPATH=\"/zeron-test/fallback/bin:/usr/bin:/bin\"; export PATH\n{RUN_PAYLOAD}"
                ),
            );
            let start = Instant::now();
            let path = snapshot_path(&shell, Duration::from_millis(400)).unwrap();
            assert!(
                path.to_string_lossy()
                    .starts_with("/zeron-test/fallback/bin"),
                "got: {}",
                path.to_string_lossy()
            );
            // First attempt burned ~400ms then was killed; the whole resolve
            // must not have waited out the sleep.
            assert!(start.elapsed() < Duration::from_secs(5));
        }

        #[test]
        fn gives_up_on_a_shell_that_never_answers() {
            let dir = tempfile::tempdir().unwrap();
            let shell = fake_shell(dir.path(), "#!/bin/sh\nsleep 60\n");
            let start = Instant::now();
            assert!(snapshot_path(&shell, Duration::from_millis(300)).is_none());
            assert!(start.elapsed() < Duration::from_secs(5));
        }
    }
}
