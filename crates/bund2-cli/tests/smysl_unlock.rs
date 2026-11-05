//! **RFC-0014 criterion 15, at process level:** `bund2 smysl-unlock <root>`
//! lists a SMYSL library's locks with their holders, and with `--yes`
//! breaks them (§S9, R14). Only in the `smysl` build.

#![cfg(feature = "smysl")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::{Command, Stdio};

fn unlock(args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("smysl-unlock")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 ran");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn it_lists_a_lock_left_by_hand_then_breaks_it() {
    let root = std::env::temp_dir().join(format!("bund2-cli-smysl-unlock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("log")).unwrap();
    let lock = root.join("lock");
    std::fs::write(&lock, "pid=4242\nhost=elsewhere\ncommand=smysl text add\n").unwrap();
    let r = root.to_str().unwrap();

    let (out, err, code) = unlock(&[r]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        format!(
            "{}: held by pid 4242 on elsewhere running `smysl text add`\n",
            lock.display()
        )
    );
    assert!(lock.exists());

    let (out, err, code) = unlock(&[r, "--yes"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        format!(
            "broke {}: held by pid 4242 on elsewhere running `smysl text add`\n",
            lock.display()
        )
    );
    assert!(!lock.exists());

    let (out, _, code) = unlock(&[r]);
    assert_eq!(code, Some(0));
    assert_eq!(out, format!("no SMYSL locks at {r}\n"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn it_asks_for_a_root() {
    let (_, err, code) = unlock(&[]);
    assert_eq!(code, Some(2));
    assert!(err.contains("bund2 smysl-unlock <root> [--yes]"), "{err}");
}
