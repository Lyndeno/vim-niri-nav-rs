use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod args;
use args::{Args, Direction, Modifier};
use clap::Parser;

#[cfg(feature = "niri-ipc")]
use niri_ipc::socket::Socket;
#[cfg(feature = "niri-ipc")]
use niri_ipc::{Action, Request, Response};
use rmpv::{decode, encode, Value};

const MAX_ANCESTOR_DEPTH: u32 = 20;

#[cfg(feature = "niri-ipc")]
impl Direction {
    fn niri_action(self, modifier: Option<Modifier>) -> Action {
        match (self, modifier) {
            (Direction::Up, None) => Action::FocusWindowUp {},
            (Direction::Up, Some(Modifier::Workspace)) => Action::FocusWindowOrWorkspaceUp {},
            (Direction::Up, Some(Modifier::Monitor)) => Action::FocusWindowOrMonitorUp {},
            (Direction::Down, None) => Action::FocusWindowDown {},
            (Direction::Down, Some(Modifier::Workspace)) => Action::FocusWindowOrWorkspaceDown {},
            (Direction::Down, Some(Modifier::Monitor)) => Action::FocusWindowOrMonitorDown {},
            (Direction::Left, _) => Action::FocusColumnLeft {},
            (Direction::Right, _) => Action::FocusColumnRight {},
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        })
    }
}

#[cfg(not(feature = "niri-ipc"))]
impl Modifier {
    fn as_niri_str(self) -> &'static str {
        match self {
            Modifier::Workspace => "or-workspace-",
            Modifier::Monitor => "or-monitor-",
        }
    }
}

fn main() {
    let args = Args::parse();

    let timeout = Duration::from_secs_f64(
        env::var("VIM_NIRI_NAV_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1_f64),
    );

    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());

    let vim_navigated = focused_pid()
        .map(|pid| try_vim_nav(&runtime_dir, pid, args.direction, timeout))
        .unwrap_or(false);

    if vim_navigated {
        return;
    }

    niri_focus(args.direction, args.modifier);
}

fn is_servername_file(name: &str) -> bool {
    name.starts_with("vim-niri-nav.") && name.ends_with(".servername")
}

fn server_for_entry(entry: fs::DirEntry, focused_pid: u32) -> Option<(String, String)> {
    let name = entry.file_name();
    let name = name.to_string_lossy();

    if !is_servername_file(&name) {
        return None;
    }

    let vim_pid: u32 = name["vim-niri-nav.".len()..name.len() - ".servername".len()]
        .parse()
        .ok()?;

    if !is_descendant(vim_pid, focused_pid) {
        return None;
    }

    let content = fs::read_to_string(entry.path()).ok()?;
    let mut parts = content.trim().splitn(2, ' ');
    let program = parts.next()?.to_owned();
    let servername = parts.next().filter(|s| !s.is_empty())?.to_owned();

    Some((program, servername))
}

fn try_vim_nav(
    runtime_dir: &str,
    focused_pid: u32,
    direction: Direction,
    timeout: Duration,
) -> bool {
    let Ok(entries) = fs::read_dir(runtime_dir) else {
        return false;
    };

    entries
        .flatten()
        .find_map(|entry| {
            let (program, servername) = server_for_entry(entry, focused_pid)?;
            let expr = format!("VimNiriNav('{direction}', 1)");
            Some(match program.as_str() {
                "nvim" => nvim_eval(&servername, &expr, timeout) == Some(true),
                "vim" => vim_remote_expr(&servername, &expr, timeout) == Some(true),
                _ => false,
            })
        })
        .unwrap_or(false)
}

#[cfg(feature = "niri-ipc")]
fn focused_pid() -> Option<u32> {
    let mut socket = Socket::connect().ok()?;
    match socket.send(Request::FocusedWindow).ok()?.ok()? {
        Response::FocusedWindow(w) => u32::try_from(w?.pid?).ok(),
        _ => None,
    }
}

#[cfg(not(feature = "niri-ipc"))]
fn focused_pid() -> Option<u32> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-window"])
        .output()
        .ok()?;
    parse_focused_pid(std::str::from_utf8(&output.stdout).ok()?)
}

#[cfg(not(feature = "niri-ipc"))]
fn parse_focused_pid(json: &str) -> Option<u32> {
    let after = json.split("\"pid\":").nth(1)?;
    after
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn ppid(pid: u32) -> Option<u32> {
    let content = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    content.lines().find_map(|line| {
        line.strip_prefix("PPid:")
            .and_then(|v| v.trim().parse().ok())
    })
}

fn is_descendant(pid: u32, ancestor: u32) -> bool {
    std::iter::successors(Some(pid), |&p| ppid(p))
        .take(MAX_ANCESTOR_DEPTH as usize)
        .any(|p| p == ancestor)
}

#[cfg(feature = "niri-ipc")]
fn niri_focus(direction: Direction, modifier: Option<Modifier>) {
    let Ok(mut socket) = Socket::connect() else {
        return;
    };
    let _ = socket.send(Request::Action(direction.niri_action(modifier)));
}

#[cfg(not(feature = "niri-ipc"))]
fn niri_focus(direction: Direction, modifier: Option<Modifier>) {
    let custom = modifier.map(Modifier::as_niri_str).unwrap_or("");
    let action = match direction {
        Direction::Up | Direction::Down => format!("focus-window-{custom}{direction}"),
        Direction::Left | Direction::Right => format!("focus-column-{direction}"),
    };
    Command::new("niri")
        .args(["msg", "action", &action])
        .status()
        .ok();
}

fn nvim_eval(socket_path: &str, expr: &str, timeout: Duration) -> Option<bool> {
    let mut stream = UnixStream::connect(socket_path).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    let request = Value::Array(vec![
        Value::Integer(0.into()),
        Value::Integer(1.into()),
        Value::String("nvim_eval".into()),
        Value::Array(vec![Value::String(expr.into())]),
    ]);

    let mut buf = Vec::new();
    encode::write_value(&mut buf, &request).ok()?;
    stream.write_all(&buf).ok()?;

    // msgpack-rpc response: [type=1, msgid, error, result]
    let response = decode::read_value(&mut stream).ok()?;
    let result = response.as_array()?.get(3)?;
    Some(match result {
        Value::Boolean(b) => *b,
        Value::String(s) => s.as_str() == Some("true"),
        Value::Integer(i) => i.as_i64() != Some(0),
        _ => false,
    })
}

fn vim_remote_expr(servername: &str, expr: &str, timeout: Duration) -> Option<bool> {
    let child = Command::new("vim")
        .args(["--servername", servername, "--remote-expr", expr])
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;

    let pid = nix::unistd::Pid::from_raw(child.id() as i32);
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = child
            .wait_with_output()
            .ok()
            .map(|o| o.stdout.starts_with(b"true"));
        tx.send(result).ok();
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => {
            let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_is_servername_file_valid() {
        assert!(is_servername_file("vim-niri-nav.123.servername"));
        assert!(is_servername_file("vim-niri-nav.99999.servername"));
    }

    #[test]
    fn test_is_servername_file_invalid() {
        assert!(!is_servername_file("other.123.servername"));
        assert!(!is_servername_file("vim-niri-nav.123.txt"));
        assert!(!is_servername_file("VIM-niri-nav.123.servername"));
    }

    #[cfg(not(feature = "niri-ipc"))]
    #[test]
    fn test_parse_focused_pid_valid() {
        let json = r#"{"id":1,"app_id":"alacritty","pid":12345}"#;
        assert_eq!(parse_focused_pid(json), Some(12345));
    }

    #[cfg(not(feature = "niri-ipc"))]
    #[test]
    fn test_parse_focused_pid_with_spaces() {
        assert_eq!(parse_focused_pid(r#"{"pid": 99}"#), Some(99));
    }

    #[cfg(not(feature = "niri-ipc"))]
    #[test]
    fn test_parse_focused_pid_missing() {
        assert_eq!(parse_focused_pid("{}"), None);
    }

    #[test]
    fn test_server_for_entry_valid() {
        let dir = tempfile::tempdir().unwrap();
        let pid = std::process::id();
        let filename = format!("vim-niri-nav.{pid}.servername");
        fs::write(dir.path().join(&filename), "nvim /tmp/nvim.sock").unwrap();

        let entry = fs::read_dir(dir.path()).unwrap().flatten().next().unwrap();

        assert_eq!(
            server_for_entry(entry, pid),
            Some(("nvim".to_owned(), "/tmp/nvim.sock".to_owned()))
        );
    }

    #[test]
    fn test_server_for_entry_wrong_filename() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("not-a-match.txt"), "nvim /tmp/nvim.sock").unwrap();

        let entry = fs::read_dir(dir.path()).unwrap().flatten().next().unwrap();

        assert_eq!(server_for_entry(entry, 1), None);
    }

    #[test]
    fn test_server_for_entry_unrelated_pid() {
        let dir = tempfile::tempdir().unwrap();
        // PID 1 is init/systemd — not a descendant of the test process
        fs::write(
            dir.path().join("vim-niri-nav.1.servername"),
            "nvim /tmp/nvim.sock",
        )
        .unwrap();

        let entry = fs::read_dir(dir.path()).unwrap().flatten().next().unwrap();

        assert_eq!(server_for_entry(entry, std::process::id()), None);
    }

    #[cfg(feature = "niri-ipc")]
    #[test]
    fn test_direction_niri_action_up() {
        use niri_ipc::Action;
        assert!(matches!(
            Direction::Up.niri_action(None),
            Action::FocusWindowUp {}
        ));
        assert!(matches!(
            Direction::Up.niri_action(Some(Modifier::Workspace)),
            Action::FocusWindowOrWorkspaceUp {}
        ));
        assert!(matches!(
            Direction::Up.niri_action(Some(Modifier::Monitor)),
            Action::FocusWindowOrMonitorUp {}
        ));
    }

    #[cfg(feature = "niri-ipc")]
    #[test]
    fn test_direction_niri_action_down() {
        use niri_ipc::Action;
        assert!(matches!(
            Direction::Down.niri_action(None),
            Action::FocusWindowDown {}
        ));
        assert!(matches!(
            Direction::Down.niri_action(Some(Modifier::Workspace)),
            Action::FocusWindowOrWorkspaceDown {}
        ));
        assert!(matches!(
            Direction::Down.niri_action(Some(Modifier::Monitor)),
            Action::FocusWindowOrMonitorDown {}
        ));
    }

    #[cfg(feature = "niri-ipc")]
    #[test]
    fn test_direction_niri_action_left_right_ignore_modifier() {
        use niri_ipc::Action;
        assert!(matches!(
            Direction::Left.niri_action(None),
            Action::FocusColumnLeft {}
        ));
        assert!(matches!(
            Direction::Left.niri_action(Some(Modifier::Monitor)),
            Action::FocusColumnLeft {}
        ));
        assert!(matches!(
            Direction::Left.niri_action(Some(Modifier::Workspace)),
            Action::FocusColumnLeft {}
        ));
        assert!(matches!(
            Direction::Right.niri_action(None),
            Action::FocusColumnRight {}
        ));
        assert!(matches!(
            Direction::Right.niri_action(Some(Modifier::Monitor)),
            Action::FocusColumnRight {}
        ));
        assert!(matches!(
            Direction::Right.niri_action(Some(Modifier::Workspace)),
            Action::FocusColumnRight {}
        ));
    }
}
