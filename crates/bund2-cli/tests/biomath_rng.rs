//! RFC-0015 criterion 11's second bullet, at process level: two `vm.spawn`
//! children seeded alike answer the same LIST (§S8). `vm.spawn` is the
//! runtime's, not the standard library's, so this is the binary's test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::{Command, Stdio};

fn run(name: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("bund2-biomath-rng-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("bund2 ran");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn children_seeded_alike_draw_alike() {
    let child =
        "{ 42 rng.seed 1.0 2.0 dist.normal 5 dist.sample* } list vm.spawn vm.join println\n";
    let out = run("seeded.bund", &child.repeat(2));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_eq!(lines[0], lines[1]);
    assert!(lines[0].matches("::").count() >= 5, "{out}");
}

/// Each child has a generator of its own, unseeded: two that are not seeded
/// do not repeat each other.
#[test]
fn unseeded_children_draw_apart() {
    let child = "{ 1.0 2.0 dist.normal 5 dist.sample* } list vm.spawn vm.join println\n";
    let out = run("unseeded.bund", &child.repeat(2));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert_ne!(lines[0], lines[1]);
}
