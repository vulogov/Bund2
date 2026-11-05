//! The `smysl.` natives registered so far (RFC-0014 §S4): M2's
//! `smysl.open` and `smysl.agent`; M3's `smysl.create` and `smysl.unit`,
//! the first that write; M4's, the rest of forming: `smysl.rel`,
//! `smysl.label`, `smysl.append`, `smysl.text.add`, `smysl.texts`,
//! `smysl.parse`, `smysl.write` and `smysl.check`; and M5's, retraction
//! and research: `smysl.retract`, `smysl.get`, `smysl.trace`,
//! `smysl.status`, `smysl.salience` and `smysl.pack`; and M6's,
//! `smysl.dossier` and `smysl.measure`, which append nothing; and M7's,
//! `smysl.query` and the constructor `smysl`, whose conditional's runner
//! is [`super::query::run_smysl`]; and M8's, `smysl.kb`.
//!
//! - **SM11 is the house text, unprefixed** (§S8). Every other refusal
//!   begins `<W>: `, the word's name in capitals.
//! - **Under `--noio`**, a word that touches disk is registered as a stub
//!   that answers SM06, `opaque(0)`, as D119 stubs `csv` (R11).
//!   `smysl.agent` and `smysl.parse` touch nothing and are not stubbed.

use bund2_api::{Error, Leaves, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, MAP, STRING};
use smysl_core::AgentId;

use super::handle::{self, Handle};
use super::{Code, Refusal, form, map, sm03};

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

/// `root smysl.create` — create a library at `root`, or open the one there,
/// as `Library::create` does (§S4).
///
/// The root's directory is made first, since a path that does not exist
/// has no canonical form; a failure is SM01's `cannot reach`. Then the root
/// is canonicalised, the process mutex taken under it, and only then
/// `Library::create` called, which tolerates the directory
/// (`smysl-text/src/library.rs:289`). It takes no SMYSL lock: there is
/// nothing to lock before the marker exists.
fn smysl_create(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.create";
    let [root] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let answer = string(&root, 1).and_then(|root| {
        let root = std::path::Path::new(&root);
        std::fs::create_dir_all(root).map_err(|e| {
            Refusal::new(Code::Sm01, format!("cannot reach {}: {e}", root.display()))
        })?;
        let canon = handle::canonical(root)?;
        let _serial = super::write::serialise(&canon);
        smysl_text::Library::create(&canon).map_err(super::sm02)?;
        handle::opened(&canon)
    });
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// The handle `#k` names, or SM00.
fn handle_at(v: &BundValue, k: usize) -> Result<Handle, Refusal> {
    Handle::from_value(v, k)
}

/// The record MAP `#k` holds, of the kind this word takes, or SM03.
fn record_of(
    v: &BundValue,
    k: usize,
    want: map::Kind,
) -> Result<std::collections::BTreeMap<String, BundValue>, Refusal> {
    let m = v.as_map().filter(|_| v.dt() == MAP).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be MAP, found {}",
            v.type_name()
        ))
    })?;
    let kind = map::kind(m)?;
    if kind != want {
        return Err(sm03(format!(
            "#{k} parameter is a {} MAP, not a {}",
            kind.name(),
            want.name()
        )));
    }
    Ok(m.clone())
}

/// Report a warning: a `Warning`, one line on stderr, prefixed with the
/// word's name in capitals (D36, §S8). Every `smysl.` warning comes
/// through here.
pub(crate) fn warn(vm: &mut dyn Vm, word: &str, text: &str) {
    vm.report(bund2_api::diag::Diagnostic::warning(format!(
        "{}: {text}",
        word.to_uppercase()
    )));
}

/// The withdrawn-edge warning (§S4), for each rid a forming word answered
/// that `is_withdrawn` reports.
fn withdrawn(vm: &mut dyn Vm, word: &str, rids: &[smysl_core::Uid]) {
    for rid in rids {
        warn(
            vm,
            word,
            &format!(
                "edge {} is withdrawn: nothing that interprets the graph follows it",
                rid.canonical()
            ),
        );
    }
}

/// Decide `items` as one batch under the locks, and append what it holds;
/// answer each item's id and the withdrawn rids answered.
fn formed(
    word: &str,
    h: &Handle,
    decide: impl FnOnce(&mut form::Batch<'_>) -> Result<Vec<smysl_core::Uid>, Refusal>,
) -> Result<(Vec<smysl_core::Uid>, Vec<smysl_core::Uid>), Refusal> {
    super::write::write(word, &h.root, |w| {
        let (ids, records, rids) = {
            let mut b = form::Batch::new(w.library.store(), &h.agent);
            let ids = decide(&mut b)?;
            let (records, rids) = b.finish();
            (ids, records, rids)
        };
        w.append(&records)?;
        Ok((ids, rids))
    })
}

/// `handle unit smysl.unit` — append one unit written as §S2's MAP, with
/// its attestation, and answer its uid (§S4).
fn smysl_unit(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.unit";
    let [unit, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 2).and_then(|h| {
        let m = record_of(&unit, 1, map::Kind::Unit)?;
        formed(W, &h, |b| Ok(vec![b.unit(&m)?]))
    });
    let (ids, _) = answer.map_err(|r| r.for_word(W))?;
    vm.push(map::uid(ids.first().ok_or_else(|| {
        Error::internal(format!("{W}: one unit was decided and none answered"))
    })?));
    Ok(())
}

/// `handle relation smysl.rel` — append one relation, with its
/// attestation, and answer its rid (§S4). A `retracts` relation is SM03
/// (R23); an edge `is_withdrawn` reports is answered and warned of.
fn smysl_rel(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.rel";
    let [rel, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 2).and_then(|h| {
        let m = record_of(&rel, 1, map::Kind::Relation)?;
        formed(W, &h, |b| Ok(vec![b.relation(&m)?]))
    });
    let (ids, rids) = answer.map_err(|r| r.for_word(W))?;
    withdrawn(vm, W, &rids);
    vm.push(map::uid(ids.first().ok_or_else(|| {
        Error::internal(format!("{W}: one relation was decided and none answered"))
    })?));
    Ok(())
}

/// `handle label uid smysl.label` — bind a label to a unit the library
/// holds (§S4). Bound to another unit already is SM08; bound to this one
/// already writes nothing.
fn smysl_label(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.label";
    let [uid, label, h] = <[BundValue; 3]>::try_from(operands(vm, W, 3)?)
        .map_err(|_| Error::internal(format!("{W}: three operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 3).and_then(|h| {
        let l = string(&label, 2)?;
        let label = smysl_core::Label::new(l.as_str())
            .map_err(|e| sm03(format!("label {l} is refused: {e}")))?;
        let u = string(&uid, 1)?;
        let uid = smysl_core::Uid::parse(&u).map_err(|_| sm03(format!("uid {u} is not a uid")))?;
        formed(W, &h, |b| Ok(vec![b.label(label, uid)?]))
    });
    answer.map_err(|r| r.for_word(W))?;
    Ok(())
}

/// `handle list smysl.append` — append a LIST of unit, relation and label
/// MAPs, in order and under one acquisition of the locks, and answer each
/// one's id (§S4). A refusal anywhere writes nothing of the LIST.
fn smysl_append(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.append";
    let [list, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 2).and_then(|h| {
        let items = list
            .as_list()
            .filter(|_| list.dt() == bund2_value::LIST)
            .ok_or_else(|| {
                sm03(format!(
                    "#1 parameter must be LIST, found {}",
                    list.type_name()
                ))
            })?;
        let maps = items
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.as_map()
                    .filter(|_| v.dt() == MAP)
                    .cloned()
                    .ok_or_else(|| {
                        sm03(format!(
                            "list item {i} must be MAP, found {}",
                            v.type_name()
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        formed(W, &h, |b| maps.iter().map(|m| b.record(m)).collect())
    });
    let (ids, rids) = answer.map_err(|r| r.for_word(W))?;
    withdrawn(vm, W, &rids);
    vm.push(BundValue::list(ids.iter().map(map::uid).collect()));
    Ok(())
}

/// `handle spec smysl.text.add` — add a text through a reader, and answer
/// `{ mid, parts, lossy, redacted }` from SMYSL's `Added`
/// (`smysl-text/src/library.rs:160-187`).
///
/// As SMYSL's `text add` does, under the locks: `complete_index` before
/// `add`, and `write_index` after it succeeds
/// (`smysl:src/main.rs:12667,12680`). A failed `write_index` is a warning,
/// as SMYSL prints it, since the sidecar is rebuilt on the next open.
fn smysl_text_add(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.text.add";
    let [spec, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 2).and_then(|h| {
        let s = super::text::spec(&spec, 1)?;
        super::write::write(W, &h.root, |w| {
            w.library.complete_index();
            let added = w.call(|lib| {
                lib.add(
                    &smysl_text::Input::new(&s.input),
                    &s.spec,
                    &smysl_text::Caps::default(),
                )
            })?;
            let index = w.library.write_index().err();
            Ok((added, index))
        })
    });
    let (added, index) = answer.map_err(|r| r.for_word(W))?;
    if let Some(e) = index {
        warn(
            vm,
            W,
            &format!("the index was not written ({e}); it will be rebuilt on open"),
        );
    }
    let mut m = std::collections::BTreeMap::new();
    m.insert("mid".to_owned(), BundValue::str(added.mid.canonical()));
    m.insert(
        "parts".to_owned(),
        BundValue::list(
            added
                .parts
                .iter()
                .map(|t| BundValue::str(t.canonical()))
                .collect(),
        ),
    );
    m.insert("lossy".to_owned(), BundValue::boolean(added.lossy));
    m.insert(
        "redacted".to_owned(),
        BundValue::int(i64::try_from(added.objects_redacted).unwrap_or(i64::MAX)),
    );
    vm.push(BundValue::map(m));
    Ok(())
}

/// The library `#k`'s handle names, opened without a lock, for a word that
/// only reads (§S3).
fn opened_at(v: &BundValue, k: usize) -> Result<smysl_text::Library, Refusal> {
    let h = handle_at(v, k)?;
    let (_, library) = handle::open(&h.root)?;
    Ok(library)
}

/// `handle smysl.texts` — each alias with its heads, a LIST of manifest ids
/// (`Library::catalog`, `smysl-text/src/library.rs:1590-1601`).
fn smysl_texts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.texts";
    let [h] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let library = opened_at(&h, 1).map_err(|r| r.for_word(W))?;
    let catalog = library
        .catalog()
        .into_iter()
        .map(|(alias, heads)| {
            (
                alias,
                BundValue::list(
                    heads
                        .iter()
                        .map(|m| BundValue::str(m.canonical()))
                        .collect(),
                ),
            )
        })
        .collect();
    vm.push(BundValue::map(catalog));
    Ok(())
}

/// `text smysl.parse` — the surface syntax as §S2's MAPs (§S4). No I/O, so
/// not stubbed under `--noio`. Warnings are reported before a refusal.
fn smysl_parse(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.parse";
    let [text] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let text = string(&text, 1).map_err(|r| r.for_word(W))?;
    let parsed = super::surface::parse(&text);
    for w in &parsed.warnings {
        warn(vm, W, w);
    }
    let maps = parsed.answer.map_err(|r| r.for_word(W))?;
    vm.push(BundValue::list(maps));
    Ok(())
}

/// `handle smysl.write` — the surface syntax of the library's units,
/// relations and labels, and a warning for each kind of record left out
/// (§S4).
fn smysl_write(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.write";
    let [h] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let library = opened_at(&h, 1).map_err(|r| r.for_word(W))?;
    let written = super::surface::write(library.store());
    for w in &written.warnings {
        warn(vm, W, w);
    }
    vm.push(BundValue::str(written.text));
    Ok(())
}

/// `handle smysl.check` — every diagnostic of `check`'s store passes, as
/// MAPs, under `CheckOptions::default()` [R19], so under the profile SMYSL
/// derives from the library's views.
fn smysl_check(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.check";
    let [h] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let library = opened_at(&h, 1).map_err(|r| r.for_word(W))?;
    let report = smysl_check::check(library.store(), smysl_check::CheckOptions::default());
    let diagnostics = report
        .diagnostics
        .iter()
        .map(map::diagnostic)
        .collect::<Result<Vec<_>, _>>()?;
    vm.push(BundValue::list(diagnostics));
    Ok(())
}

fn noio(_vm: &mut dyn Vm) -> Result<(), Error> {
    Err(Refusal::new(Code::Sm06, NOIO).for_word("smysl"))
}

/// Operand `#k` as a uid, or SM03.
fn uid_at(v: &BundValue, k: usize) -> Result<smysl_core::Uid, Refusal> {
    let u = string(v, k)?;
    smysl_core::Uid::parse(&u).map_err(|_| sm03(format!("uid {u} is not a uid")))
}

/// `handle uid smysl.retract` — retract a unit as SMYSL's `retract` does,
/// and answer the rid of the self-edge written; NODATA when it was
/// retracted directly already, and nothing is written (§S4). A plan
/// `Origin` authority refuses is SM09.
fn smysl_retract(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.retract";
    let [uid, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 2).and_then(|h| {
        let target = uid_at(&uid, 1)?;
        super::write::write(W, &h.root, |w| {
            match form::retract(w.library.store(), &h.agent, target)? {
                form::Retraction::Already => Ok(None),
                form::Retraction::Edge(rel) => {
                    let rid = rel.uid();
                    w.append(&[smysl_core::Record::Relation(rel)])?;
                    Ok(Some(rid))
                }
            }
        })
    });
    let answer = answer.map_err(|r| r.for_word(W))?;
    vm.push(answer.as_ref().map_or_else(BundValue::nodata, map::uid));
    Ok(())
}

/// `handle uid smysl.get` — the unit as §S2's MAP, NODATA if absent (§S5).
fn smysl_get(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.get";
    let [uid, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer = opened_at(&h, 2).and_then(|library| {
        let uid = uid_at(&uid, 1)?;
        Ok(super::research::get(library.store(), &uid))
    });
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// `handle uid kind depth smysl.trace` — the lineage (§S5).
fn smysl_trace(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.trace";
    let [depth, kind, uid, h] = <[BundValue; 4]>::try_from(operands(vm, W, 4)?)
        .map_err(|_| Error::internal(format!("{W}: four operands were guarded and not pulled")))?;
    let answer = handle_at(&h, 4).and_then(|_| {
        let root = uid_at(&uid, 3)?;
        let walk = super::research::walk(&string(&kind, 2)?)?;
        let depth = super::research::depth(&depth, 1)?;
        let library = opened_at(&h, 4)?;
        Ok(super::research::lineage(library.store(), root, walk, depth))
    });
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// `handle smysl.status` — effective status under the strict policy, for
/// every unit whose status changed (§S5).
fn smysl_status(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.status";
    let [h] = <[BundValue; 1]>::try_from(operands(vm, W, 1)?)
        .map_err(|_| Error::internal(format!("{W}: one operand was guarded and not pulled")))?;
    let library = opened_at(&h, 1).map_err(|r| r.for_word(W))?;
    vm.push(super::research::status(library.store()));
    Ok(())
}

/// `handle spec smysl.salience` — `{ uid, score }`, highest first (§S5).
fn smysl_salience(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.salience";
    let [spec, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer =
        opened_at(&h, 2).and_then(|library| super::research::ranked(library.store(), &spec, 1));
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// `handle spec smysl.pack` — the uids `pack` selects to `spec.budget`
/// tokens (§S5).
fn smysl_pack(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.pack";
    let [spec, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let answer =
        opened_at(&h, 2).and_then(|library| super::research::packed(library.store(), &spec, 1));
    vm.push(answer.map_err(|r| r.for_word(W))?);
    Ok(())
}

/// Report each diagnostic as a warning, as SMYSL's `Display` writes it
/// (§S8, D36): a run's warnings and a measure's diagnostics, whatever
/// their severity.
pub(crate) fn warn_each(vm: &mut dyn Vm, word: &str, diagnostics: &[smysl_core::Diagnostic]) {
    for d in diagnostics {
        warn(vm, word, &d.to_string());
    }
}

/// `handle question smysl.dossier` — the dossier `ask` assembles, as
/// `{ subject, entities, slots }`; its slots' warnings are reported, and
/// nothing is appended (§S5).
fn smysl_dossier(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.dossier";
    let [question, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let asked = handle_at(&h, 2).and_then(|h| {
        let question = string(&question, 1)?;
        let (root, library) = handle::open(&h.root)?;
        super::research::dossier(&library, &root, &h.agent, &question)
    });
    let asked = asked.map_err(|r| r.for_word(W))?;
    warn_each(vm, W, &asked.warnings);
    vm.push(asked.answer);
    Ok(())
}

/// `handle ids scope smysl.measure` — one MAP per id, `{ id, ok, value,
/// uid, diagnostics }`; each result's diagnostics are reported, and
/// nothing is appended (§S5).
fn smysl_measure(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.measure";
    let [scope, ids, h] = <[BundValue; 3]>::try_from(operands(vm, W, 3)?)
        .map_err(|_| Error::internal(format!("{W}: three operands were guarded and not pulled")))?;
    let ran = handle_at(&h, 3).and_then(|h| {
        let ids = super::research::ids(&ids, 2)?;
        let scope = string(&scope, 1)?;
        let (root, library) = handle::open(&h.root)?;
        super::research::measure(&library, &root, &ids, &scope)
    });
    let ran = ran.map_err(|r| r.for_word(W))?;
    let mut answers = Vec::with_capacity(ran.len());
    for r in &ran {
        let m = super::research::measured(r)?;
        warn_each(vm, W, &m.warnings);
        answers.push(m.answer);
    }
    vm.push(BundValue::list(answers));
    Ok(())
}

/// `handle text smysl.query` — one `sq/1` query's answer as §S6's MAP; its
/// warnings are reported (§S6, §S8).
fn smysl_query(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.query";
    let [text, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let q = handle_at(&h, 2).and_then(|h| {
        let text = string(&text, 1)?;
        let (root, library) = handle::open(&h.root)?;
        super::query::query(&library, &root, &text)
    });
    let q = q.map_err(|r| r.for_word(W))?;
    warn_each(vm, W, &q.warnings);
    vm.push(super::query::answer(&q.answer)?);
    Ok(())
}

/// `handle scope smysl.kb` — a new KB of the units the scope selects, as
/// §S7's facts; the query's warnings are reported (§S7, §S8).
fn smysl_kb(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "smysl.kb";
    let [text, h] = <[BundValue; 2]>::try_from(operands(vm, W, 2)?)
        .map_err(|_| Error::internal(format!("{W}: two operands were guarded and not pulled")))?;
    let loaded = handle_at(&h, 2).and_then(|h| {
        let text = string(&text, 1)?;
        let (root, library) = handle::open(&h.root)?;
        let q = super::query::query(&library, &root, &text)?;
        let store = library.store();
        let scope = super::facts::scope(store, &q.answer.uids());
        Ok((super::facts::facts(store, &scope), q.warnings))
    });
    let (facts, warnings) = loaded.map_err(|r| r.for_word(W))?;
    warn_each(vm, W, &warnings);
    vm.push(super::facts::kb(&facts)?);
    Ok(())
}

/// `smysl` — a CONDITIONAL of type `smysl`, to fill with `set` and run with
/// `!` (§S6).
fn smysl_word(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(crate::conditional::new_conditional("smysl"));
    Ok(())
}

/// The words that touch disk, stubbed under `--noio` (R11).
const ON_DISK: [&str; 20] = [
    "smysl.append",
    "smysl.check",
    "smysl.create",
    "smysl.dossier",
    "smysl.get",
    "smysl.kb",
    "smysl.label",
    "smysl.measure",
    "smysl.open",
    "smysl.pack",
    "smysl.query",
    "smysl.rel",
    "smysl.retract",
    "smysl.salience",
    "smysl.status",
    "smysl.text.add",
    "smysl.texts",
    "smysl.trace",
    "smysl.unit",
    "smysl.write",
];

pub fn register(r: &mut Registry, opts: &crate::host::HostOptions) {
    let leaving = |c, tag| StackEffect::fixed(c, 1).leaving(Leaves::Tag(tag));
    let handle = |c| leaving(c, MAP);
    if opts.noio {
        for name in ON_DISK {
            r.register_native(name, noio, StackEffect::opaque(0), WordKind::Sync);
        }
        // The constructor reads nothing, and is stubbed with the handler,
        // as D119 stubs `csv` and its handler: a CONDITIONAL built with
        // `conditional` reaches the handler without the constructor (R11).
        r.register_native("smysl", noio, StackEffect::opaque(0), WordKind::Sync);
        r.register_conditional("smysl", |vm, _c| noio(vm));
    } else {
        r.register_native("smysl.create", smysl_create, handle(1), WordKind::Sync);
        r.register_native("smysl.open", smysl_open, handle(1), WordKind::Sync);
        r.register_native("smysl.unit", smysl_unit, leaving(2, STRING), WordKind::Sync);
        r.register_native("smysl.rel", smysl_rel, leaving(2, STRING), WordKind::Sync);
        r.register_native(
            "smysl.label",
            smysl_label,
            StackEffect::fixed(3, 0),
            WordKind::Sync,
        );
        r.register_native(
            "smysl.append",
            smysl_append,
            leaving(2, LIST),
            WordKind::Sync,
        );
        r.register_native(
            "smysl.text.add",
            smysl_text_add,
            leaving(2, MAP),
            WordKind::Sync,
        );
        r.register_native("smysl.texts", smysl_texts, leaving(1, MAP), WordKind::Sync);
        r.register_native(
            "smysl.write",
            smysl_write,
            leaving(1, STRING),
            WordKind::Sync,
        );
        r.register_native("smysl.check", smysl_check, leaving(1, LIST), WordKind::Sync);
        // A rid, or NODATA: no one tag.
        let fixed = |c| StackEffect::fixed(c, 1);
        r.register_native("smysl.retract", smysl_retract, fixed(2), WordKind::Sync);
        r.register_native("smysl.get", smysl_get, fixed(2), WordKind::Sync);
        r.register_native("smysl.trace", smysl_trace, leaving(4, LIST), WordKind::Sync);
        r.register_native(
            "smysl.status",
            smysl_status,
            leaving(1, MAP),
            WordKind::Sync,
        );
        r.register_native(
            "smysl.salience",
            smysl_salience,
            leaving(2, LIST),
            WordKind::Sync,
        );
        r.register_native("smysl.pack", smysl_pack, leaving(2, LIST), WordKind::Sync);
        r.register_native(
            "smysl.dossier",
            smysl_dossier,
            leaving(2, MAP),
            WordKind::Sync,
        );
        r.register_native(
            "smysl.measure",
            smysl_measure,
            leaving(3, LIST),
            WordKind::Sync,
        );
        r.register_native("smysl.query", smysl_query, leaving(2, MAP), WordKind::Sync);
        r.register_native("smysl.kb", smysl_kb, leaving(2, MAP), WordKind::Sync);
        // D181: it leaves a CONDITIONAL of the type its runner is filed under.
        r.register_native(
            "smysl",
            smysl_word,
            StackEffect::fixed(0, 1).leaving(Leaves::Conditional("smysl")),
            WordKind::Sync,
        );
        r.register_conditional("smysl", super::query::run_smysl);
    }
    r.register_native("smysl.agent", smysl_agent, handle(2), WordKind::Sync);
    r.register_native("smysl.parse", smysl_parse, leaving(1, LIST), WordKind::Sync);
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

    // --- M3: the first writes -------------------------------------------------

    fn library(tag: &str) -> (Dir, String) {
        let dir = Dir::fresh(tag);
        smysl_text::Library::create(&dir.0).unwrap();
        let canon = std::fs::canonicalize(&dir.0).unwrap();
        let root = canon.to_str().unwrap().to_owned();
        (dir, root)
    }

    fn handle_of(root: &str, agent: &str) -> BundValue {
        let mut m = std::collections::BTreeMap::new();
        m.insert("type".to_owned(), BundValue::str("smysl"));
        m.insert("root".to_owned(), BundValue::str(root));
        m.insert("agent".to_owned(), BundValue::str(agent));
        BundValue::map(m)
    }

    fn unit_map(gist: &str) -> BundValue {
        let mut m = std::collections::BTreeMap::new();
        m.insert("schema".to_owned(), BundValue::str("claim"));
        m.insert("gist".to_owned(), BundValue::str(gist));
        m.insert("status".to_owned(), BundValue::str("speculative"));
        BundValue::map(m)
    }

    /// `smysl.unit` of `unit` through a handle on `root` as `agent`.
    fn append_unit(root: &str, agent: &str, unit: BundValue) -> Result<String, String> {
        use bund2_api::Vm as _;
        let mut i = interp(false);
        i.push(handle_of(root, agent));
        i.push(unit);
        run_in(&mut i, "smysl.unit").map(|v| v.as_str().unwrap())
    }

    fn reopen(root: &str) -> smysl_text::Library {
        smysl_text::Library::open(std::path::Path::new(root)).unwrap()
    }

    fn uid_of(s: &str) -> smysl_core::Uid {
        smysl_core::Uid::parse(s).unwrap()
    }

    #[test]
    fn create_makes_a_library_and_opens_one_already_there() {
        let dir = Dir::fresh("create");
        let h = run(&format!("{} smysl.create", quoted(&dir.0))).unwrap();
        let canon = std::fs::canonicalize(&dir.0).unwrap();
        assert_eq!(field(&h, "root"), canon.to_str().unwrap());
        assert!(smysl_text::Library::is_library(&canon));
        let again = run(&format!("{} smysl.create", quoted(&dir.0))).unwrap();
        assert_eq!(again.display(), h.display());
    }

    /// 14: `smysl.create` of a root beneath a regular file, which
    /// `create_dir_all` cannot make, is SM01's `cannot reach`.
    #[test]
    fn create_beneath_a_regular_file_cannot_reach_it() {
        let dir = Dir::fresh("create-file");
        std::fs::create_dir_all(&dir.0).unwrap();
        let file = dir.0.join("plain");
        std::fs::write(&file, b"x").unwrap();
        let root = file.join("lib");
        let err = run(&format!("{} smysl.create", quoted(&root))).unwrap_err();
        assert!(
            err.starts_with(&format!("SMYSL.CREATE: cannot reach {}: ", root.display())),
            "{err}"
        );
    }

    /// Criterion 4: for each unit of a `.smy` fixture, the uid `smysl.unit`
    /// answers for it written as §S2's MAP is the uid SMYSL computes, by
    /// `parse_surface` and `canonical_uid` called directly.
    #[test]
    fn uids_agree_with_smysls_own_parser() {
        const FIXTURE: &str = r#"@schema x.text/v1 { version: 1, types: ["x.text/entity"], relations: [x.text/mentions] }

@x.text/entity n/naomi { status: speculative, source: { kind: doc, ref: "alias:kjv/ruth" } }
~ Naomi

@x.text/entity n/noemin { status: speculative, lang: ru, source: { kind: doc, ref: "alias:synodal/ruth" } }
~ Ноеминь

@definition d/naomi { status: cited, source: { kind: doc, ref: "Ruth 1:2" } }
~ Naomi is the wife of Elimelech, a woman of Bethlehem-judah.

@claim c/empty { status: inferred, grounds: [o/return] }
~ Naomi came back to Bethlehem with nothing.

@observation o/return { status: cited, source: { kind: doc, ref: "Ruth 1:21" } }
~ Naomi said: I went out full, and the LORD hath brought me home again empty.

@question q/kinsman { status: speculative, n: 3, tags: ["kin", "field"] }
~ Would Naomi's kinsman redeem the field of Elimelech?
"#;
        let parsed = smysl_core::surface::parse_surface(FIXTURE).unwrap();
        assert!(!parsed.has_errors(), "{:?}", parsed.diagnostics);
        let (_dir, root) = library("crit4");
        let mut n = 0;
        for r in &parsed.records {
            let smysl_core::Record::Unit(core) = r else {
                continue;
            };
            let mut m = super::map::unit(core, &[]).as_map().unwrap().clone();
            m.remove("uid");
            assert!(!m.contains_key("opaque"), "{m:?}");
            let answered = append_unit(&root, "tool:bund2", BundValue::map(m)).unwrap();
            assert_eq!(answered, smysl_core::canonical_uid(core).canonical());
            n += 1;
        }
        assert_eq!(n, 6);
    }

    /// §S4's attestation (R15): one per unit, `Authored`, rung `Document`
    /// for a cited unit and `Computed` otherwise, hop 0, no parents, the
    /// handle's agent; a re-append adds nothing; another agent's unit gains
    /// this agent's attestation.
    #[test]
    fn a_unit_is_written_with_one_attestation_and_once() {
        use smysl_core::{Op, Rung};
        let (_dir, root) = library("attest");
        let u = uid_of(&append_unit(&root, "tool:bund2", unit_map("The well is dry")).unwrap());
        let lib = reopen(&root);
        assert!(lib.store().contains_uid(&u));
        let a: Vec<_> = lib.store().attestations_of(&u).iter().cloned().collect();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].agent.as_str(), "tool:bund2");
        assert_eq!(
            (a[0].op, a[0].rung, a[0].hop),
            (Op::Authored, Rung::Computed, 0)
        );
        assert!(a[0].parents.is_empty());
        let log = std::path::Path::new(&root).join(smysl_text::library::CATALOG);
        let len = std::fs::metadata(&log).unwrap().len();

        // Again, by the same agent: nothing reaches the log.
        let again = append_unit(&root, "tool:bund2", unit_map("The well is dry")).unwrap();
        assert_eq!(uid_of(&again), u);
        assert_eq!(std::fs::metadata(&log).unwrap().len(), len);

        // By another: one more attestation, and the unit once.
        append_unit(&root, "tool:other", unit_map("The well is dry")).unwrap();
        let lib = reopen(&root);
        let agents: Vec<String> = lib
            .store()
            .attestations_of(&u)
            .iter()
            .map(|a| a.agent.as_str().to_owned())
            .collect();
        assert_eq!(agents.len(), 2, "{agents:?}");
        assert!(agents.contains(&"tool:other".to_owned()));

        // An agent's clocks never go backwards: the second attestation by
        // `tool:bund2` advances from its first.
        let second = uid_of(&append_unit(&root, "tool:bund2", unit_map("Another")).unwrap());
        let lib = reopen(&root);
        let first_ts = &lib
            .store()
            .attestations_of(&u)
            .iter()
            .find(|a| a.agent.as_str() == "tool:bund2")
            .unwrap()
            .ts;
        let second_ts = &lib
            .store()
            .attestations_of(&second)
            .iter()
            .next()
            .unwrap()
            .ts;
        assert!(second_ts > first_ts);
    }

    #[test]
    fn a_cited_unit_is_attested_at_the_document_rung() {
        let (_dir, root) = library("cited");
        let mut m = unit_map("Naomi").as_map().unwrap().clone();
        m.insert("status".to_owned(), BundValue::str("cited"));
        let mut src = std::collections::BTreeMap::new();
        src.insert("kind".to_owned(), BundValue::str("doc"));
        src.insert("reference".to_owned(), BundValue::str("Ruth 1:2"));
        m.insert("source".to_owned(), BundValue::map(src));
        let u = uid_of(&append_unit(&root, "tool:bund2", BundValue::map(m)).unwrap());
        let lib = reopen(&root);
        let a = lib
            .store()
            .attestations_of(&u)
            .iter()
            .next()
            .unwrap()
            .clone();
        assert_eq!(a.rung, smysl_core::Rung::Document);
    }

    /// SM02 on a write: a unit SMYSL refuses to build, here a gist of spaces
    /// (`smysl-core/src/types/unit.rs:146-149`), carries SMYSL's reason,
    /// writes nothing and leaves no lock.
    #[test]
    fn a_unit_smysl_refuses_is_sm02_and_leaves_no_lock() {
        let (_dir, root) = library("sm02");
        let err = append_unit(&root, "tool:bund2", unit_map("   ")).unwrap_err();
        assert!(err.starts_with("SMYSL.UNIT: SMYSL refused it: "), "{err}");
        assert!(!err.contains("left locked"), "{err}");
        assert!(!std::path::Path::new(&root).join("lock").exists());
        append_unit(&root, "tool:bund2", unit_map("then a real one")).unwrap();
    }

    /// SM08: a `grounds` entry naming an unbound label.
    #[test]
    fn an_unbound_label_in_grounds_is_sm08() {
        let (_dir, root) = library("sm08");
        let mut m = unit_map("Rests on nothing").as_map().unwrap().clone();
        m.insert("status".to_owned(), BundValue::str("inferred"));
        m.insert(
            "grounds".to_owned(),
            BundValue::list(vec![BundValue::str("d/nowhere")]),
        );
        assert_eq!(
            append_unit(&root, "tool:bund2", BundValue::map(m)).unwrap_err(),
            "SMYSL.UNIT: label d/nowhere binds 0 units"
        );
    }

    #[test]
    fn unit_refuses_a_relation_map_and_a_non_map() {
        let (_dir, root) = library("kinds");
        let mut rel = std::collections::BTreeMap::new();
        for k in ["kind", "from", "to"] {
            rel.insert(k.to_owned(), BundValue::str("x"));
        }
        assert_eq!(
            append_unit(&root, "tool:bund2", BundValue::map(rel)).unwrap_err(),
            "SMYSL.UNIT: #1 parameter is a relation MAP, not a unit"
        );
        assert_eq!(
            append_unit(&root, "tool:bund2", BundValue::int(3)).unwrap_err(),
            "SMYSL.UNIT: #1 parameter must be MAP, found Integer"
        );
    }

    /// Criterion 15: with a lock file left as a crashed SMYSL process would
    /// leave it, a write is SM07 naming the holder, and `smysl.open` of the
    /// library succeeds.
    #[test]
    fn a_stale_lock_is_sm07_and_reads_still_succeed() {
        let (_dir, root) = library("stale");
        let lock = std::path::Path::new(&root).join("lock");
        std::fs::write(&lock, "pid=4242\nhost=elsewhere\ncommand=smysl text add\n").unwrap();
        let expected = smysl_core::LibError::Locked {
            what: "library".to_owned(),
            held_by: "pid 4242 on elsewhere running `smysl text add`".to_owned(),
        };
        assert_eq!(
            append_unit(&root, "tool:bund2", unit_map("blocked")).unwrap_err(),
            format!("SMYSL.UNIT: the library is locked: {expected}")
        );
        run(&format!("{root:?} smysl.open")).unwrap();
        assert!(lock.exists());
    }

    /// Criterion 15: four VMs on four threads each append 250 units to one
    /// library, through two spellings of its root, both absolute.
    #[test]
    fn four_threads_append_a_thousand_units() {
        let (dir, root) = library("threads");
        let canon = std::path::PathBuf::from(&root);
        let other = canon.join("..").join(canon.file_name().unwrap());
        let spellings = [root.clone(), other.to_str().unwrap().to_owned()];
        let handles: Vec<_> = (0..4)
            .map(|t| {
                let root = spellings[t % 2].clone();
                std::thread::spawn(move || {
                    for n in 0..250 {
                        append_unit(&root, "tool:bund2", unit_map(&format!("unit {t}/{n}")))
                            .unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let lib = reopen(&root);
        let units = lib
            .store()
            .iter()
            .filter(|r| matches!(r, smysl_core::Record::Unit(_)))
            .count();
        assert_eq!(units, 1000);
        drop(dir);
    }

    /// 18: `smysl.create` and `smysl.open` of the same new root on two
    /// threads, repeated, never answer anything but a handle, SM01, or SM02
    /// naming the layout.
    #[test]
    fn create_and_open_race_to_a_handle_or_a_named_refusal() {
        for n in 0..25 {
            let dir = Dir::fresh(&format!("race-{n}"));
            let root = format!("{:?}", dir.0.to_str().unwrap());
            // A value holds an `Rc`, so each thread answers its handle's
            // type, not the handle.
            let a = {
                let root = root.clone();
                std::thread::spawn(move || {
                    run(&format!("{root} smysl.create")).map(|h| field(&h, "type"))
                })
            };
            let b = std::thread::spawn(move || {
                run(&format!("{root} smysl.open")).map(|h| field(&h, "type"))
            });
            for r in [a.join().unwrap(), b.join().unwrap()] {
                match r {
                    Ok(kind) => assert_eq!(kind, "smysl"),
                    Err(e) => assert!(
                        e.starts_with("SMYSL.OPEN: no SMYSL library at ")
                            || e.starts_with("SMYSL.OPEN: cannot reach ")
                            || (e.contains("SMYSL refused it: ") && e.contains("layout")),
                        "{e}"
                    ),
                }
            }
        }
    }

    #[test]
    fn under_noio_the_writing_words_are_sm06() {
        assert_eq!(
            run_in(&mut interp(true), "\"/nowhere\" smysl.create").unwrap_err(),
            NOIO
        );
        assert_eq!(
            run_over(&mut interp(true), handle_value("/nowhere"), "smysl.unit").unwrap_err(),
            NOIO
        );
    }

    /// Criterion 15, with the probe armed to `panic`: the write is reported
    /// as `Error::internal`, leaves every lock it took, makes the next write
    /// from a fresh VM on another thread SM07 naming this process, and
    /// leaves `smysl.open` succeeding. `bund2 smysl-unlock`'s function then
    /// lists each lock with its holder, breaks them, and a write succeeds.
    #[cfg(feature = "smysl-panic-probe")]
    #[test]
    fn a_panic_inside_a_write_leaves_the_locks() {
        use super::super::probe::{Action, arm};
        use super::super::unlock::unlock;
        let (_dir, root) = library("panic");
        arm(std::path::Path::new(&root), Action::Panic);
        let err = {
            use bund2_api::Vm as _;
            let mut i = interp(false);
            i.push(handle_of(&root, "tool:bund2"));
            i.push(unit_map("never written"));
            let code = bund2_syntax::compile("smysl.unit").unwrap();
            i.eval(&code).unwrap_err()
        };
        assert!(err.is_internal() || err.0.contains("internal"), "{}", err.0);
        assert!(err.0.contains("smysl-panic-probe"), "{}", err.0);
        let rootp = std::path::Path::new(&root);
        assert!(rootp.join("lock").exists());
        assert!(rootp.join("log").join("catalog.lock").exists());

        let r = root.clone();
        let next = std::thread::spawn(move || append_unit(&r, "tool:bund2", unit_map("later")))
            .join()
            .unwrap()
            .unwrap_err();
        assert!(
            next.starts_with("SMYSL.UNIT: the library is locked: "),
            "{next}"
        );
        assert!(
            next.contains(&format!("pid {}", std::process::id())),
            "{next}"
        );
        run(&format!("{root:?} smysl.open")).unwrap();

        let found = unlock(rootp, false).unwrap();
        assert_eq!(found.len(), 2);
        for f in &found {
            assert_eq!(f.holder.as_ref().unwrap().pid, std::process::id());
            assert_eq!(f.holder.as_ref().unwrap().command, "bund2 smysl.unit");
        }
        unlock(rootp, true).unwrap();
        append_unit(&root, "tool:bund2", unit_map("after the unlock")).unwrap();
    }
}
