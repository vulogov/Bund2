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
    used_url(&format!("http://127.0.0.1:{origin}/lib.bund"), vars)
}

/// [`used`] for any URL.
fn used_url(url: &str, vars: &[(&str, String)]) -> String {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let run = RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bund2-proxy-{}-{run}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("s.bund");
    std::fs::write(&path, format!("\"{url}\" use\n")).expect("script");
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

/// As [`serve`], keeping the head of every request it is sent, a `CONNECT`'s
/// included.
fn serve_recording(tag: &'static str) -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    let heads = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = heads.clone();
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
                seen.lock().expect("heads").push(String::from_utf8_lossy(&head).to_lowercase());
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
    (port, heads)
}

/// **The fourteenth review's B1 and B2**: with a proxy set, a URL the
/// reference refuses is not fetched, and a proxy Bund2 cannot log in to as
/// the reference would is not asked. Before, the proxy was sent a `CONNECT`
/// for `a!b.invalid` and its answer was evaluated. Each row is one the
/// oracle refused, or fetched from somewhere else, on 2026-10-09.
#[test]
fn a_url_the_reference_refuses_reaches_no_listener() {
    let (origin, at_origin) = serve_recording("origin");
    let (proxy, at_proxy) = serve_recording("proxy");
    let through = [("http_proxy", format!("http://127.0.0.1:{proxy}"))];
    // A refusal is a report, and the report is on standard output.
    let refused = |url: &str, vars: &[(&str, String)]| used_url(url, vars).contains("USE can not get from");
    for host in ["a!b.invalid", "a,b.invalid", "a+b.invalid", "127.0.0.1!", "[zz]", "[1.2.3.4]"] {
        assert!(refused(&format!("http://{host}:{origin}/x"), &through), "{host}");
    }
    for user in ["a%00b", "u:%00"] {
        let url = format!("http://{user}@127.0.0.1:{origin}/x");
        assert!(refused(&url, &through), "{user} through a proxy");
        assert!(refused(&url, &[]), "{user} directly");
    }
    for user in ["a%00b", "a%0ab", "u:%1f", "a%40b"] {
        let vars = [("http_proxy", format!("http://{user}@127.0.0.1:{proxy}"))];
        assert!(refused(&format!("http://127.0.0.1:{origin}/x"), &vars), "proxy user {user}");
    }
    assert_eq!(*at_origin.lock().expect("heads"), Vec::<String>::new(), "the origin was asked");
    assert_eq!(*at_proxy.lock().expect("heads"), Vec::<String>::new(), "the proxy was asked");
    // And the listeners do answer.
    assert_eq!(used_url(&format!("http://a~b.invalid:{origin}/x"), &through), "proxy");
}

/// **The fourteenth review's S1 and S4**: a URL's credentials are sent
/// decoded, as the oracle sent them, and an IPv6 host in libcurl's text.
#[test]
fn a_request_carries_what_the_reference_sends() {
    let (origin, heads) = serve_recording("origin");
    let last = || heads.lock().expect("heads").last().cloned().unwrap_or_default();
    // `a@b:c:d`, `u s:` and `a<b:` in base64.
    for (user, basic) in [("a%40b:c%3Ad", "yubiomm6za=="), ("u%20s", "dsbzog=="), ("a<b", "ytxiog==")] {
        assert_eq!(used_url(&format!("http://{user}@127.0.0.1:{origin}/x"), &[]), "origin", "{user}");
        assert!(last().contains(&format!("authorization: basic {basic}\r\n")), "{user}: {}", last());
    }
    assert_eq!(used_url(&format!("http://127.0.0.1:{origin}/x"), &[]), "origin");
    assert!(!last().contains("authorization"), "no user part, no header: {}", last());
    assert_eq!(used_url(&format!("http://127.1:{origin}/x"), &[]), "origin");
    assert!(last().contains(&format!("host: 127.0.0.1:{origin}\r\n")), "{}", last());
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
