//! **F182 at process level: which environment variables route a fetch.**
//!
//! `bund2-stdlib` tests the choice with the environment passed in. This
//! spawns the binary with the variables really set, because the defect was a
//! default nobody had passed anything to: `ureq` read the environment itself.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

/// Answer every request with a Bund program that prints `tag`. A `CONNECT` is
/// granted first, which is how `ureq` asks a proxy for anything.
fn serve(tag: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { break };
            loop {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match conn.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                if head.starts_with(b"CONNECT ") {
                    let _ = conn.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                    continue;
                }
                let body = format!("\"{tag}\" println\n");
                let _ = write!(
                    conn,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                break;
            }
        }
    });
    port
}

/// `use` a library from `origin` with `vars` set and no other proxy variable,
/// and return what the program printed.
fn used(origin: u16, vars: &[(&str, String)]) -> String {
    let dir = std::env::temp_dir().join(format!("bund2-proxy-{}-{origin}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("s.bund");
    std::fs::write(&path, format!("\"http://127.0.0.1:{origin}/lib.bund\" use\n")).expect("script");
    let mut c = Command::new(env!("CARGO_BIN_EXE_bund2"));
    for name in [
        "http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY",
        "no_proxy", "NO_PROXY",
    ] {
        c.env_remove(name);
    }
    c.envs(vars.iter().map(|(k, v)| (*k, v.as_str())));
    c.args(["script", "--file"]).arg(&path);
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = c.output().expect("bund2 runs");
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// As [`serve`], on the one port libcurl takes for a proxy that names none.
/// `None` when something else on this machine already has it.
fn serve_on_1080() -> Option<()> {
    let listener = TcpListener::bind("127.0.0.1:1080").ok()?;
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { break };
            loop {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match conn.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                if head.starts_with(b"CONNECT ") {
                    let _ = conn.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                    continue;
                }
                let body = "\"p1080\" println\n";
                let _ = write!(
                    conn,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                break;
            }
        }
    });
    Some(())
}

/// **The twelfth review's B1**: a proxy that names no port is on 1080, as
/// libcurl has it, and not on `ureq`'s 80. The port is fixed, so this checks
/// nothing on a machine where 1080 is taken, and says so on standard error,
/// past the harness's capture; the string Bund2
/// hands `ureq` is checked wherever the tests run, in `bund2-stdlib`.
#[test]
fn a_proxy_that_names_no_port_is_asked_on_1080() {
    let origin = serve("origin");
    if serve_on_1080().is_none() {
        // Written to the descriptor and not through `eprintln!`, which the
        // harness captures and discards for a test that passes.
        let _ = std::io::stderr()
            .write_all(b"\nfetch_proxy: port 1080 is in use; the wire check of a port-less proxy DID NOT RUN\n");
        return;
    }
    for proxy in ["127.0.0.1", "http://127.0.0.1", "http://localhost", "http://127.0.0.1/"] {
        assert_eq!(used(origin, &[("http_proxy", proxy.to_string())]), "p1080", "{proxy}");
    }
}

#[test]
fn a_fetch_obeys_the_proxy_variables_the_reference_obeys_and_no_others() {
    let origin = serve("origin");
    let proxy = serve("proxy");
    let there = format!("http://127.0.0.1:{proxy}");

    assert_eq!(used(origin, &[]), "origin");
    for obeyed in ["http_proxy", "all_proxy", "ALL_PROXY"] {
        assert_eq!(used(origin, &[(obeyed, there.clone())]), "proxy", "{obeyed}");
    }
    // The three `ureq` read by default and libcurl does not, for `http://`.
    for ignored in ["HTTP_PROXY", "HTTPS_PROXY", "https_proxy"] {
        assert_eq!(used(origin, &[(ignored, there.clone())]), "origin", "{ignored}");
    }
    assert_eq!(
        used(origin, &[("http_proxy", there.clone()), ("NO_PROXY", "127.0.0.1".into())]),
        "origin"
    );
}
