use std::env;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{Parser, ValueEnum};

const MAX_ANCESTOR_DEPTH: u32 = 20;

#[derive(Parser)]
#[command(about = "Navigate between vim splits and niri windows")]
struct Args {
    direction: Direction,
    modifier: Option<Modifier>,
}

#[derive(ValueEnum, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn niri_action(self, modifier: Option<Modifier>) -> String {
        let custom = modifier.map(Modifier::as_niri_str).unwrap_or("");
        match self {
            Direction::Up | Direction::Down => format!("focus-window-{custom}{self}"),
            Direction::Left | Direction::Right => format!("focus-column-{self}"),
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

#[derive(ValueEnum, Clone, Copy)]
enum Modifier {
    #[value(name = "w")]
    Workspace,
    #[value(name = "m")]
    Monitor,
}

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

fn try_vim_nav(runtime_dir: &str, focused_pid: u32, direction: Direction, timeout: Duration) -> bool {
    let entries = match fs::read_dir(runtime_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    entries.flatten().find_map(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if !is_servername_file(&name) {
            return None;
        }

        let vim_pid: u32 = name["vim-niri-nav.".len()..name.len() - ".servername".len()].parse().ok()?;

        if !is_descendant(vim_pid, focused_pid) {
            return None;
        }

        let content = fs::read_to_string(entry.path()).ok()?;
        let mut parts = content.trim().splitn(2, ' ');
        let program = parts.next().unwrap_or("");
        let servername = parts.next().unwrap_or("");

        if servername.is_empty() {
            return None;
        }

        let expr = format!("VimNiriNav('{direction}', 1)");
        Some(match program {
            "nvim" => nvim_eval(servername, &expr, timeout) == Some(true),
            "vim" => vim_remote_expr(servername, &expr, timeout) == Some(true),
            _ => false,
        })
    }).unwrap_or(false)
}

fn focused_pid() -> Option<u32> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-window"])
        .output()
        .ok()?;
    parse_focused_pid(std::str::from_utf8(&output.stdout).ok()?)
}

// Parses "pid": <number> without pulling in serde_json
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

fn niri_focus(direction: Direction, modifier: Option<Modifier>) {
    Command::new("niri")
        .args(["msg", "action", &direction.niri_action(modifier)])
        .status()
        .ok();
}

fn nvim_eval(socket_path: &str, expr: &str, timeout: Duration) -> Option<bool> {
    let mut stream = UnixStream::connect(socket_path).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    stream.write_all(&encode_request(expr)).ok()?;

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).ok()?;

    Some(buf[..n].windows(4).any(|w| w == b"true"))
}

fn vim_remote_expr(servername: &str, expr: &str, timeout: Duration) -> Option<bool> {
    let (tx, rx) = mpsc::channel();
    let servername = servername.to_string();
    let expr = expr.to_string();

    thread::spawn(move || {
        let result = Command::new("vim")
            .args(["--servername", &servername, "--remote-expr", &expr])
            .output()
            .ok()
            .map(|o| o.stdout.starts_with(b"true"));
        tx.send(result).ok();
    });

    rx.recv_timeout(timeout).ok().flatten()
}

fn encode_request(expr: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(0x94); // fixarray(4)
    buf.push(0x00); // request type
    buf.push(0x01); // msgid
    encode_str(&mut buf, "nvim_eval");
    buf.push(0x91); // fixarray(1)
    encode_str(&mut buf, expr);
    buf
}

fn encode_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    match bytes.len() {
        n @ 0..=31 => buf.push(0xa0 | n as u8),
        n @ 32..=255 => {
            buf.push(0xd9);
            buf.push(n as u8);
        }
        n => {
            buf.push(0xda);
            buf.push((n >> 8) as u8);
            buf.push(n as u8);
        }
    }
    buf.extend_from_slice(bytes);
}
