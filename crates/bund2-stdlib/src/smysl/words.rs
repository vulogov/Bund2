//! The `smysl.` natives registered so far (RFC-0014 §S4): M2's
//! `smysl.open` and `smysl.agent`.
//!
//! - **SM11 is the house text, unprefixed** (§S8). Every other refusal
//!   begins `<W>: `, the word's name in capitals.
//! - **Under `--noio`**, a word that touches disk is registered as a stub
//!   that answers SM06, `opaque(0)`, as D119 stubs `csv` (R11).
//!   `smysl.agent` touches nothing and is not stubbed.

use bund2_api::{Error, Leaves, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, MAP};
use smysl_core::AgentId;

use super::handle::{self, Handle};
use super::{Code, Refusal, sm03};

/// SM06's text, the house pattern of `bund SAVE.MODEL functions disabled
/// with --noio` (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:125`).
pub const NOIO: &str = "bund SMYSL functions disabled with --noio";

/// SM11, and the `n` operands it guards, `#1` first.
fn operands(vm: &mut dyn Vm, word: &str, n: usize) -> Result<Vec<BundValue>, Error> {
    if vm.depth() < n {
        return Err(Refusal::new(
            Code::Sm11,
            format!("Stack is too shallow for inline {word}"),
        )
        .for_word(word));
    }
    (0..n)
        .map(|_| {
            vm.pull().ok_or_else(|| {
                Error::internal(format!(
                    "{word}: the stack was deep enough and gave nothing"
                ))
            })
        })
        .collect()
}

/// Operand `#k` as a STRING, or SM03.
fn string(v: &BundValue, k: usize) -> Result<String, Refusal> {
    v.str_ref()
        .filter(|_| v.dt() == bund2_value::STRING)
        .map(str::to_owned)
        .ok_or_else(|| {
            sm03(format!(
                "#{k} parameter must be STRING, found {}",
                v.type_name()
            ))
        })
}

/// `root smysl.open` — the handle of the library at `root`, its root
/// canonicalised and its agent the default; SM01 if there is none (§S4).
fn smysl_open(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.open";
    let [root] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let answer = string(&root, 1).and_then(|root| {
        let (canon, _library) = handle::open(std::path::Path::new(&root))?;
        handle::opened(&canon)
    });
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// `handle agent smysl.agent` — the handle with another agent; SM03 if the
/// agent does not match SMYSL's grammar. No I/O (§S2, R11).
fn smysl_agent(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.agent";
    let [agent, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = Handle::from_value(&h, 2).and_then(|h| {
        let agent = string(&agent, 1)?;
        let agent = AgentId::new(agent.as_str())
            .map_err(|_| sm03(format!("agent {agent} does not match SMYSL's grammar")))?;
        Ok(h.with_agent(agent).to_value())
    });
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

fn noio(_vm: &mut dyn Vm) -> Result<(), Error> {
    Err(Refusal::new(Code::Sm06, NOIO).for_word("smysl"))
}

pub fn register(r: &mut Registry, opts: &crate::host::HostOptions) {
    let handle = |c| StackEffect::fixed(c, 1).leaving(Leaves::Tag(MAP));
    if opts.noio {
        r.register_native("smysl.open", noio, StackEffect::opaque(0), WordKind::Sync);
    } else {
        r.register_native("smysl.open", smysl_open, handle(1), WordKind::Sync);
    }
    r.register_native("smysl.agent", smysl_agent, handle(2), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bund2_interp::Interp;

    use super::*;

    /// A fresh directory under the system's temporary one, removed on drop.
    struct Dir(PathBuf);

    impl Dir {
        fn fresh(tag: &str) -> Dir {
            static N: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "bund2-smysl-words-{}-{tag}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&p);
            Dir(p)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn interp(noio: bool) -> Interp {
        let mut i = Interp::new();
        crate::register_all_with(
            &mut i.registry,
            &crate::host::HostOptions {
                noio,
                ..Default::default()
            },
        );
        i
    }

    /// Run `src`, answering the top of the stack or the error's text.
    fn run_in(i: &mut Interp, src: &str) -> Result<BundValue, String> {
        let code = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
        // The interpreter wraps a word's error; the word's own text is
        // what follows its last ` returned error: `.
        i.eval(&code).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })?;
        i.snapshot()
            .last()
            .cloned()
            .ok_or_else(|| "empty".to_owned())
    }

    fn run(src: &str) -> Result<BundValue, String> {
        run_in(&mut interp(false), src)
    }

    fn quoted(p: &std::path::Path) -> String {
        format!("{:?}", p.to_str().unwrap())
    }

    fn field(v: &BundValue, key: &str) -> String {
        v.as_map().unwrap()[key].as_str().unwrap()
    }

    #[test]
    fn open_answers_a_handle_with_the_canonical_root_and_the_default_agent() {
        let dir = Dir::fresh("open");
        smysl_text::Library::create(&dir.0).unwrap();
        let h = run(&format!("{} smysl.open", quoted(&dir.0))).unwrap();
        assert_eq!(field(&h, "type"), "smysl");
        assert_eq!(field(&h, "agent"), "tool:bund2");
        let canon = std::fs::canonicalize(&dir.0).unwrap();
        assert_eq!(field(&h, "root"), canon.to_str().unwrap());

        // Another spelling of the same directory gives the same handle.
        let dotted = dir.0.join("..").join(dir.0.file_name().unwrap());
        let again = run(&format!("{} smysl.open", quoted(&dotted))).unwrap();
        assert_eq!(again.display(), h.display());
    }

    #[test]
    fn open_refuses_with_sm01_where_there_is_no_library() {
        let dir = Dir::fresh("none");
        assert_eq!(
            run(&format!("{} smysl.open", quoted(&dir.0))).unwrap_err(),
            format!("SMYSL.OPEN: no SMYSL library at {}", dir.0.display())
        );
        // A directory that exists but holds no marker is no library either.
        std::fs::create_dir_all(&dir.0).unwrap();
        assert_eq!(
            run(&format!("{} smysl.open", quoted(&dir.0))).unwrap_err(),
            format!("SMYSL.OPEN: no SMYSL library at {}", dir.0.display())
        );
    }

    #[test]
    fn open_beneath_a_regular_file_cannot_reach_it() {
        let dir = Dir::fresh("file");
        std::fs::create_dir_all(&dir.0).unwrap();
        let file = dir.0.join("plain");
        std::fs::write(&file, b"x").unwrap();
        let root = file.join("lib");
        let err = run(&format!("{} smysl.open", quoted(&root))).unwrap_err();
        assert!(
            err.starts_with(&format!("SMYSL.OPEN: cannot reach {}: ", root.display())),
            "{err}"
        );
    }

    /// SM02 by a primary shard `smysl.open` cannot read. Phase 1 builds
    /// without `store-redb`, so the fixture is a marker made by hand beside
    /// where the shard's log would be, `log/shards/<stem>.primary`
    /// (`primary_marker` and `log_path`, `smysl-text/src/shards.rs:42,65-71,115-119`),
    /// which `shards::list` and `is_primary` read and `read_primary` refuses
    /// (`:96-99,188-190,306-322`).
    #[test]
    fn open_of_a_primary_shard_this_build_cannot_read_is_sm02() {
        let dir = Dir::fresh("primary");
        smysl_text::Library::create(&dir.0).unwrap();
        let shards = dir.0.join("log").join("shards");
        std::fs::create_dir_all(&shards).unwrap();
        std::fs::write(shards.join("extra.primary"), b"").unwrap();
        let err = run(&format!("{} smysl.open", quoted(&dir.0))).unwrap_err();
        assert!(err.starts_with("SMYSL.OPEN: SMYSL refused it: "), "{err}");
    }

    #[test]
    fn open_of_a_non_string_is_sm03() {
        assert_eq!(
            run("7 smysl.open").unwrap_err(),
            "SMYSL.OPEN: #1 parameter must be STRING, found Integer"
        );
    }

    #[test]
    fn a_shallow_stack_is_the_house_text() {
        assert_eq!(
            run("smysl.open").unwrap_err(),
            "Stack is too shallow for inline smysl.open"
        );
        assert_eq!(
            run("\"tool:x\" smysl.agent").unwrap_err(),
            "Stack is too shallow for inline smysl.agent"
        );
    }

    fn handle_value(root: &str) -> BundValue {
        let mut m = std::collections::BTreeMap::new();
        m.insert("type".to_owned(), BundValue::str("smysl"));
        m.insert("root".to_owned(), BundValue::str(root));
        m.insert("agent".to_owned(), BundValue::str("tool:bund2"));
        BundValue::map(m)
    }

    /// Run `src` over a stack that already holds `under`.
    fn run_over(i: &mut Interp, under: BundValue, src: &str) -> Result<BundValue, String> {
        use bund2_api::Vm as _;
        i.push(under);
        run_in(i, src)
    }

    #[test]
    fn agent_replaces_the_agent_and_touches_no_file() {
        // A root that does not exist: `smysl.agent` asks nothing of it.
        let dir = Dir::fresh("agent");
        let root = dir.0.to_str().unwrap();
        let h = run_over(
            &mut interp(false),
            handle_value(root),
            "\"human:ann\" smysl.agent",
        )
        .unwrap();
        assert_eq!(field(&h, "agent"), "human:ann");
        assert_eq!(field(&h, "root"), root);
        assert!(!dir.0.exists());
    }

    #[test]
    fn agent_refuses_a_bad_handle_and_a_bad_agent() {
        // 14's relative-root case: SM00, and no file touched.
        assert_eq!(
            run_over(
                &mut interp(false),
                handle_value("rel/lib"),
                "\"tool:x\" smysl.agent"
            )
            .unwrap_err(),
            "SMYSL.AGENT: #2 parameter must be a SMYSL handle"
        );
        assert!(!std::path::Path::new("rel").exists());
        assert_eq!(
            run("\"/a\" \"tool:x\" smysl.agent").unwrap_err(),
            "SMYSL.AGENT: #2 parameter must be a SMYSL handle"
        );

        let good = || handle_value("/nowhere/lib");
        assert_eq!(
            run_over(&mut interp(false), good(), "\"Not An Agent\" smysl.agent").unwrap_err(),
            "SMYSL.AGENT: agent Not An Agent does not match SMYSL's grammar"
        );
        assert_eq!(
            run_over(&mut interp(false), good(), "7 smysl.agent").unwrap_err(),
            "SMYSL.AGENT: #1 parameter must be STRING, found Integer"
        );
    }

    #[test]
    fn under_noio_open_is_sm06_and_agent_still_works() {
        let mut i = interp(true);
        assert_eq!(run_in(&mut i, "\"/nowhere\" smysl.open").unwrap_err(), NOIO);
        let h = run_over(
            &mut interp(true),
            handle_value("/nowhere/lib"),
            "\"tool:other\" smysl.agent",
        )
        .unwrap();
        assert_eq!(field(&h, "agent"), "tool:other");
    }
}
