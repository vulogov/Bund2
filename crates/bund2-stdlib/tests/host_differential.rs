//! **Temporary** — D152. `bund2-host` against `sys_metrics`, the crate it
//! replaces, read on the same machine a moment apart. `sys_metrics` is a
//! dev-dependency for this file alone and is linked into nothing shipped;
//! both go once this has passed on Linux as well as macOS.

#![allow(clippy::expect_used, clippy::print_stderr)]

#[test]
fn the_new_readings_are_the_old_ones() {
    let u = bund2_host::uname().expect("uname");
    assert_eq!(
        u.hostname,
        sys_metrics::host::get_hostname().expect("hostname")
    );
    assert_eq!(
        u.kernel_version,
        sys_metrics::host::get_kernel_version().expect("kernel")
    );
    assert_eq!(
        u.os_version,
        sys_metrics::host::get_os_version().expect("os")
    );
    assert_eq!(
        u.system,
        sys_metrics::host::get_host_info().expect("host").system
    );
    assert_eq!(
        format!("{:?}", bund2_host::virtualization()),
        format!("{:?}", sys_metrics::virt::get_virt_info())
    );
    let (new, old) = (
        bund2_host::memory().expect("memory"),
        sys_metrics::memory::get_memory().expect("memory"),
    );
    eprintln!("new {new:?}\nold {old:?}");
    assert_eq!(new.total, old.total);
    assert_eq!((new.shared, new.buffers), (old.shared, old.buffers));
    // Read a moment apart, so the moving figures agree to within a little.
    for (what, a, b) in [
        ("free", new.free, old.free),
        ("used", new.used, old.used),
        ("cached", new.cached, old.cached),
    ] {
        assert!(a.abs_diff(b) <= 64, "{what}: new {a}, old {b}");
    }
}
