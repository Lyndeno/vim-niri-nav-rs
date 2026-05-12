use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <socket> <expr> <timeout_secs>", args[0]);
        std::process::exit(1);
    }

    let socket_path = &args[1];
    let expr = &args[2];
    let timeout_secs: f64 = args[3].parse().unwrap_or(0.1);
    let timeout = Duration::from_secs_f64(timeout_secs);

    let result = eval(socket_path, expr, timeout).unwrap_or(false);
    println!("{}", if result { "true" } else { "false" });
}

fn eval(socket_path: &str, expr: &str, timeout: Duration) -> Option<bool> {
    let mut stream = UnixStream::connect(socket_path).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    stream.write_all(&encode_request(expr)).ok()?;

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).ok()?;

    // Response is [1, msgid, error, result]; check raw bytes for "true"
    Some(buf[..n].windows(4).any(|w| w == b"true"))
}

// Encodes [0, 1, "nvim_eval", [expr]] as msgpack
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
