//! The world file, and the two words that reach it: `save.model` and
//! `load.model`.
//!
//! **D27: redb, not SQLite.** The reference keeps a program's world in a SQLite
//! database (`reference/Bund/src/stdlib/helpers/world/mod.rs:19-39`). Bund2
//! keeps it in a redb file, and D31 established that nothing outside Bund reads
//! one. What goes in is unchanged: each model is the value serialised as the
//! reference serialises it (`bund2_value::wire`, D20). So a model moved between
//! the two engines needs only a change of container, not a change of bytes.
//!
//! The reference's `MODELS` table has an `INTEGER PRIMARY KEY`, a name and a
//! blob (`reference/Bund/src/stdlib/helpers/world/models.rs:10-16`). Here it is a
//! redb table from a `u64` key to the name and the blob. The key counts up from
//! 1 in the order models are saved, which is the order the reference's
//! `SELECT` returns its rows in.
//!
//! **The file's name is cleaned as the reference cleans it**, by the same
//! `sanitize-filename`. Path separators are removed, so every world file lands
//! in the working directory whatever the program asked for, and `.world` is
//! added unless the name already ends with it (`mod.rs:20-30`).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, TableError};

use crate::host::HostOptions;

/// `MODELS`: save order to `(name, serialised value)`.
const MODELS: TableDefinition<u64, (&str, &[u8])> = TableDefinition::new("MODELS");

/// The path a world name opens (`reference/Bund/src/stdlib/helpers/world/mod.rs:20-30`).
fn world_file(name: &str) -> String {
    let clean = sanitize_filename::sanitize_with_options(
        name,
        sanitize_filename::Options {
            truncate: true,
            windows: true,
            replacement: "",
        },
    );
    if clean.ends_with(".world") {
        clean
    } else {
        format!("{clean}.world")
    }
}

/// Open the world file, creating it if it does not exist, as the reference's
/// `Connection::open` does (`mod.rs:31-38`).
fn open_world(name: &str) -> Result<Database, Error> {
    Database::create(world_file(name))
        .map_err(|e| Error(format!("Open world operation returns: {e}")))
}

fn pull_string(vm: &mut dyn Vm, word: &str, n: u8) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{word} returns NO DATA #{n}")))?;
    v.as_str().ok_or_else(|| {
        Error(format!(
            "{word} casting string returns: This Dynamic type is not string"
        ))
    })
}

/// `save.model` — store a value in the world under a name
/// (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:64-122`,
/// `reference/Bund/src/stdlib/helpers/world/models.rs:77-105`).
///
/// The file is on top, the value beneath it, and the name beneath that, so the
/// source reads `<name> <value> <file> save.model`. Both strings are trimmed
/// (`bund_models.rs:102-103`). A second model under the same name is a second
/// row; nothing is replaced.
fn save_model(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for SAVE.MODEL".into()));
    }
    let file = pull_string(vm, "SAVE.MODEL", 1)?;
    let model = vm
        .pull()
        .ok_or_else(|| Error("SAVE.MODEL returns NO DATA #2".into()))?;
    // The reference numbers this pull `#2` as well (`bund_models.rs:92`).
    let name = pull_string(vm, "SAVE.MODEL", 2)?;
    let (name, file) = (name.trim().to_string(), file.trim().to_string());
    let db = open_world(&file)?;
    let fail = |e: String| Error(format!("SAVE.MODEL returns: {e}"));
    let bytes = bund2_value::wire::to_binary(&model)
        .map_err(|e| fail(format!("Error compiling mode: {e}")))?;
    let saving = |e: &dyn std::fmt::Display| fail(format!("Saving model returns: {e}"));
    let txn = db.begin_write().map_err(|e| saving(&e))?;
    {
        let mut table = txn
            .open_table(MODELS)
            .map_err(|e| fail(format!("Creating models table returns: {e}")))?;
        let next = table
            .last()
            .map_err(|e| saving(&e))?
            .map_or(1, |(k, _)| k.value() + 1);
        table
            .insert(next, (name.as_str(), bytes.as_slice()))
            .map_err(|e| saving(&e))?;
    }
    txn.commit().map_err(|e| saving(&e))?;
    Ok(())
}

/// `load.model` — push the models stored in a world
/// (`reference/Bund/src/stdlib/functions/bund/bund_models.rs:10-61`,
/// `reference/Bund/src/stdlib/helpers/world/models.rs:9-73`).
///
/// **It ignores the name** (F110). The reference selects every row
/// (`models.rs:22`) and pushes each model in save order, and it uses the name
/// only in the error it reports when there are none (`:69-71`). So
/// `"anything" "w" load.model` pushes every model in `w.world`. A row that
/// cannot be read ends the scan, as the reference's loop does (`:57-61`). A
/// model that cannot be decoded fails the word, leaving the models before it
/// on the stack.
fn load_model(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for LOAD.MODEL".into()));
    }
    let file = pull_string(vm, "LOAD.MODEL", 1)?;
    let name = pull_string(vm, "LOAD.MODEL", 2)?;
    let (name, file) = (name.trim().to_string(), file.trim().to_string());
    let db = open_world(&file)?;
    let fail = |e: String| Error(format!("LOAD.MODEL returns: {e}"));
    let txn = db
        .begin_read()
        .map_err(|e| fail(format!("Error performing MODEL select: {e}")))?;
    let mut loaded = 0usize;
    match txn.open_table(MODELS) {
        Ok(table) => {
            let rows = table
                .range(0u64..)
                .map_err(|e| fail(format!("Error performing MODEL select: {e}")))?;
            for row in rows {
                let Ok((_, v)) = row else { break };
                let (_, blob) = v.value();
                let value = bund2_value::wire::from_binary(blob)
                    .map_err(|e| fail(format!("Error recovering the body of the model: {e}")))?;
                vm.push(value);
                loaded += 1;
            }
        }
        // A world with no models yet has no table: nothing to load.
        Err(TableError::TableDoesNotExist(_)) => {}
        Err(e) => return Err(fail(format!("Creating models table returns: {e}"))),
    }
    if loaded == 0 {
        return Err(fail(format!(
            "Error loading model. None found with that name: {name}"
        )));
    }
    Ok(())
}

pub fn register(r: &mut Registry, opts: &HostOptions) {
    // `reference/Bund/src/stdlib/functions/bund/bund_models.rs:140-146`. Both
    // `--noio` stubs say `SAVE.MODEL`, `load.model`'s included (`:124-130`).
    if opts.noio {
        for name in ["save.model", "load.model"] {
            r.register_native(
                name,
                |_vm| Err(Error("bund SAVE.MODEL functions disabled with --noio".into())),
                StackEffect::opaque(0),
                WordKind::Sync,
            );
        }
    } else {
        r.register_native(
            "save.model",
            save_model,
            StackEffect::fixed(3, 0),
            WordKind::Sync,
        );
        // Opaque: it pushes as many values as the world holds.
        r.register_native("load.model", load_model, StackEffect::opaque(2), WordKind::Sync);
    }

    // --- `save`/`load` and their families ---------------------------------
    //
    // **`--noio` stubs them with two different messages**, as the reference
    // does: the save family says `bund SAVE functions disabled with --noio`
    // and the load family `bund LOAD functions disabled with --noio`
    // (`bund_save.rs`, `bund_load.rs`).
    fn s_all(vm: &mut dyn Vm) -> Result<(), Error> {
        save_world(vm, Family::All)
    }
    fn s_aliases(vm: &mut dyn Vm) -> Result<(), Error> {
        save_world(vm, Family::Aliases)
    }
    fn s_lambdas(vm: &mut dyn Vm) -> Result<(), Error> {
        save_world(vm, Family::Lambdas)
    }
    fn s_stacks(vm: &mut dyn Vm) -> Result<(), Error> {
        save_world(vm, Family::Stacks)
    }
    fn l_all(vm: &mut dyn Vm) -> Result<(), Error> {
        load_world(vm, Family::All)
    }
    fn l_aliases(vm: &mut dyn Vm) -> Result<(), Error> {
        load_world(vm, Family::Aliases)
    }
    fn l_lambdas(vm: &mut dyn Vm) -> Result<(), Error> {
        load_world(vm, Family::Lambdas)
    }
    fn l_stacks(vm: &mut dyn Vm) -> Result<(), Error> {
        load_world(vm, Family::Stacks)
    }

    if opts.noio {
        for name in ["save", "save.aliases", "save.lambdas", "save.stacks"] {
            r.register_native(
                name,
                |_vm| Err(Error("bund SAVE functions disabled with --noio".into())),
                StackEffect::opaque(0),
                WordKind::Sync,
            );
        }
        for name in [
            "load",
            "load.aliases",
            "load.lambdas",
            "load.stacks",
            "bootstrap",
            "save.script",
            "load.script",
        ] {
            r.register_native(
                name,
                |_vm| Err(Error("bund LOAD functions disabled with --noio".into())),
                StackEffect::opaque(0),
                WordKind::Sync,
            );
        }
    } else {
        // **Opaque, all of them.** `save` reads every stack by name, which
        // D55's audit calls observing beyond its operands, and `load` pushes
        // as many values as a world holds. Neither is a pair.
        r.register_native("save", s_all, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("save.aliases", s_aliases, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("save.lambdas", s_lambdas, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("save.stacks", s_stacks, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("load", l_all, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("load.aliases", l_aliases, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("load.lambdas", l_lambdas, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("load.stacks", l_stacks, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("bootstrap", bootstrap, StackEffect::opaque(1), WordKind::Sync);
        r.register_native("save.script", save_script, StackEffect::opaque(3), WordKind::Sync);
        r.register_native("load.script", load_script, StackEffect::opaque(2), WordKind::Sync);
    }

    // **`alias=` is not an I/O word**, so no flag stubs it.
    r.register_native("alias=", alias_get, StackEffect::fixed(1, 1), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;

    #[test]
    fn a_world_name_is_cleaned_as_the_reference_cleans_it() {
        assert_eq!(super::world_file("my/models"), "mymodels.world");
        assert_eq!(super::world_file("m.world"), "m.world");
    }

    /// Save two models, load under a name that matches neither: both come
    /// back, in save order (F110).
    #[test]
    fn load_model_pushes_every_model_in_save_order() {
        // The cleaner strips path separators, so a world file always lands in
        // the working directory. The name is this process's own, so no other
        // test can see it, and the file is removed afterwards. The working
        // directory is not changed, because tests share it across threads.
        let world = format!("bund2-world-test-{}", std::process::id());
        let file = format!("{world}.world");
        let _ = std::fs::remove_file(&file);
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let src = format!(
            "\"a\" 1 \"{world}\" save.model \"b\" \"two\" \"{world}\" save.model \"zz\" \"{world}\" load.model"
        );
        let stream = bund2_syntax::compile(&src).expect("compiles");
        let r = i.eval(&stream);
        let _ = std::fs::remove_file(&file);
        r.expect("runs");
        let got: Vec<String> = i.snapshot().iter().map(|v| v.display()).collect();
        assert_eq!(got, vec!["1".to_string(), "two".to_string()]);
    }
}

// ---------------------------------------------------------------------------
// `save`/`load` and their four families — the rest of `bund/bund`'s world.
// ---------------------------------------------------------------------------

/// `ALIASES`: alias to target (`…/world/aliases.rs`).
const ALIASES: TableDefinition<&str, &str> = TableDefinition::new("ALIASES");
/// `LAMBDAS`: name to the serialised body (`…/world/lambdas.rs`).
const LAMBDAS: TableDefinition<&str, &[u8]> = TableDefinition::new("LAMBDAS");
/// `STACKS`: a name per stack that exists (`…/world/stacks.rs`).
const STACKS: TableDefinition<&str, ()> = TableDefinition::new("STACKS");
/// `STACK_DATA`: `(stack, position)` to the serialised value.
const STACK_DATA: TableDefinition<(&str, u64), &[u8]> = TableDefinition::new("STACK_DATA");
/// `BOOTSTRAP`: a script's name to its source (`…/world/bootstrap.rs`).
const BOOTSTRAP: TableDefinition<&str, &str> = TableDefinition::new("BOOTSTRAP");

/// Which families a `save` or `load` touches — the reference's
/// `helpers::world::WorldFunctions`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    All,
    Aliases,
    Lambdas,
    Stacks,
}

fn write_err(what: &str, e: impl std::fmt::Display) -> Error {
    Error(format!("{what} returns: {e}"))
}

/// **`DROP TABLE` then `CREATE`, per family.** Every saver replaces its table
/// wholesale rather than merging, so a world file's aliases are exactly the
/// ones the last `save.aliases` saw. redb has no `DROP`, so the table is
/// opened and cleared, which leaves the same state.
fn replace_aliases(db: &Database, vm: &dyn Vm) -> Result<(), Error> {
    let tx = db.begin_write().map_err(|e| write_err("Saving alias", e))?;
    {
        let mut t = tx
            .open_table(ALIASES)
            .map_err(|e| write_err("Creating aliases table", e))?;
        t.retain(|_, _| false)
            .map_err(|e| write_err("Dropping aliases table", e))?;
        for (alias, target) in vm.alias_pairs() {
            t.insert(alias.as_str(), target.as_str())
                .map_err(|e| write_err("Saving alias", e))?;
        }
    }
    tx.commit().map_err(|e| write_err("Saving alias", e))
}

fn replace_lambdas(db: &Database, vm: &dyn Vm) -> Result<(), Error> {
    let tx = db.begin_write().map_err(|e| write_err("Saving lambda", e))?;
    {
        let mut t = tx
            .open_table(LAMBDAS)
            .map_err(|e| write_err("Creating lambdas table", e))?;
        t.retain(|_, _| false)
            .map_err(|e| write_err("Dropping lambdas table", e))?;
        for name in vm.lambda_names() {
            let Some(body) = vm.get_lambda(&name) else {
                continue;
            };
            // The same wire bytes the reference stores (D20): a world file
            // moved between the two engines needs a new container, not new
            // bytes.
            let blob = bund2_value::wire::to_binary(&body).map_err(Error)?;
            t.insert(name.as_str(), blob.as_slice())
                .map_err(|e| write_err("Saving lambda", e))?;
        }
    }
    tx.commit().map_err(|e| write_err("Saving lambda", e))
}

/// **Two tables, as the reference has two.** `STACKS` names them and
/// `STACK_DATA` holds the values by position, so a stack that exists and is
/// empty survives a round trip — which one table keyed by value could not
/// express.
fn replace_stacks(db: &Database, vm: &dyn Vm) -> Result<(), Error> {
    let tx = db.begin_write().map_err(|e| write_err("Saving stack", e))?;
    {
        let mut names = tx
            .open_table(STACKS)
            .map_err(|e| write_err("Creating stacks table", e))?;
        names
            .retain(|_, _| false)
            .map_err(|e| write_err("Dropping stacks table", e))?;
        let mut data = tx
            .open_table(STACK_DATA)
            .map_err(|e| write_err("Creating stack data table", e))?;
        data.retain(|_, _| false)
            .map_err(|e| write_err("Dropping stack data table", e))?;
        for name in vm.stack_names() {
            names
                .insert(name.as_str(), ())
                .map_err(|e| write_err("Saving stack", e))?;
            for (pos, v) in vm.snapshot_of(&name).iter().enumerate() {
                let blob = bund2_value::wire::to_binary(v).map_err(Error)?;
                data.insert((name.as_str(), pos as u64), blob.as_slice())
                    .map_err(|e| write_err("Saving stack data", e))?;
            }
        }
    }
    tx.commit().map_err(|e| write_err("Saving stack", e))
}

/// **`save` primes the bootstrap table by emptying it — F148.**
///
/// The reference's `bootstrap::init` is `DROP TABLE IF EXISTS BOOTSTRAP`
/// followed by a `CREATE`, and `save` calls it *last*. So **every `save`
/// destroys every script a previous `save.script` stored**, and because
/// `save.script` cannot create the table itself, the only working order is
/// `save` then `save.script` — and the next `save` undoes it. Measured, and
/// preserved rather than repaired: F148 explains why repairing it would be a
/// deviation.
fn prime_bootstrap(db: &Database) -> Result<(), Error> {
    let tx = db
        .begin_write()
        .map_err(|e| write_err("Priming bootstrap", e))?;
    {
        let mut t = tx
            .open_table(BOOTSTRAP)
            .map_err(|e| write_err("Creating bootstrap table", e))?;
        t.retain(|_, _| false)
            .map_err(|e| write_err("Dropping bootstrap table", e))?;
    }
    tx.commit().map_err(|e| write_err("Priming bootstrap", e))
}

fn load_aliases(db: &Database, vm: &mut dyn Vm) -> Result<(), Error> {
    let tx = db
        .begin_read()
        .map_err(|e| Error(format!("Error performing ALIASES select: {e}")))?;
    let t = match tx.open_table(ALIASES) {
        Ok(t) => t,
        // **A missing table is nothing to load, not a failure.** The
        // reference's `SELECT` on an absent table *does* fail, but `load` is
        // reached only through a world a `save` wrote, and redb has no empty
        // schema to read. A world with no aliases loads none.
        Err(TableError::TableDoesNotExist(_)) => return Ok(()),
        Err(e) => return Err(Error(format!("Error compiling ALIASES select: {e}"))),
    };
    let rows = t
        .iter()
        .map_err(|e| Error(format!("Error performing ALIASES select: {e}")))?;
    for row in rows {
        let (alias, target) =
            row.map_err(|e| Error(format!("Error getting ALIAS row: {e}")))?;
        vm.register_alias(alias.value(), target.value());
    }
    Ok(())
}

fn load_lambdas(db: &Database, vm: &mut dyn Vm) -> Result<(), Error> {
    let tx = db
        .begin_read()
        .map_err(|e| Error(format!("Error performing LAMBDAS select: {e}")))?;
    let t = match tx.open_table(LAMBDAS) {
        Ok(t) => t,
        Err(TableError::TableDoesNotExist(_)) => return Ok(()),
        Err(e) => return Err(Error(format!("Error compiling LAMBDAS select: {e}"))),
    };
    let rows = t
        .iter()
        .map_err(|e| Error(format!("Error performing LAMBDAS select: {e}")))?;
    for row in rows {
        let (name, blob) = row.map_err(|e| Error(format!("Error getting LAMBDA row: {e}")))?;
        let body = bund2_value::wire::from_binary(blob.value())
            .map_err(|e| Error(format!("Error converting from binary: {e}")))?;
        vm.register_lambda(name.value(), body);
    }
    Ok(())
}

/// **Loaded onto the stacks it names, bottom first.**
///
/// `push_to` tags each value with the stack it lands on, which is what the
/// reference's own push does — so a value's `stack` tag after a load names
/// where it is rather than where it was saved from. The two agree because the
/// names agree.
fn load_stacks(db: &Database, vm: &mut dyn Vm) -> Result<(), Error> {
    let tx = db
        .begin_read()
        .map_err(|e| Error(format!("Error executing SELECT for stacks: {e}")))?;
    let names = match tx.open_table(STACKS) {
        Ok(t) => t,
        Err(TableError::TableDoesNotExist(_)) => return Ok(()),
        Err(e) => return Err(Error(format!("Error compiling SELECT for stacks: {e}"))),
    };
    let data = match tx.open_table(STACK_DATA) {
        Ok(t) => t,
        Err(TableError::TableDoesNotExist(_)) => return Ok(()),
        Err(e) => {
            return Err(Error(format!(
                "Error compiling SELECT for stack data: {e}"
            )))
        }
    };
    let rows = names
        .iter()
        .map_err(|e| Error(format!("Error executing SELECT for stacks: {e}")))?;
    for row in rows {
        let (name, _) = row.map_err(|e| Error(format!("Error getting stack row: {e}")))?;
        let name = name.value().to_string();
        vm.ensure_stack(&name);
        let span = data
            .range((name.as_str(), 0u64)..=(name.as_str(), u64::MAX))
            .map_err(|e| Error(format!("Error executing SELECT for stack data: {e}")))?;
        for cell in span {
            let (_, blob) =
                cell.map_err(|e| Error(format!("Error getting stack data row: {e}")))?;
            let v = bund2_value::wire::from_binary(blob.value())
                .map_err(|e| Error(format!("Error converting from binary: {e}")))?;
            vm.push_to(&name, v);
        }
    }
    Ok(())
}

/// Every bootstrap script, in name order — what `bootstrap` runs.
fn read_all_bootstrap(db: &Database) -> Result<Vec<String>, Error> {
    let tx = db
        .begin_read()
        .map_err(|e| Error(format!("Error performing SCRIPT select: {e}")))?;
    let t = match tx.open_table(BOOTSTRAP) {
        Ok(t) => t,
        Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(Error(format!("Error compiling BOOTSTRAP select: {e}"))),
    };
    let rows = t
        .iter()
        .map_err(|e| Error(format!("Error performing SCRIPT select: {e}")))?;
    let mut out = Vec::new();
    for row in rows {
        let (_, script) = row.map_err(|e| Error(format!("Error getting SCRIPT row: {e}")))?;
        out.push(script.value().to_string());
    }
    Ok(out)
}

/// `save` and its three families.
///
/// **The error texts are copy-pasted in the reference and are preserved.**
/// `bund_save` reports `Aliases SAVE returns:` when the *lambdas* or the
/// *stacks* saver fails — one string pasted three times
/// (`reference/Bund/src/stdlib/functions/bund/bund_save.rs`). A reader of a
/// failing `save` is told the wrong family, and that is what the reference
/// says.
fn save_world(vm: &mut dyn Vm, family: Family) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for SAVE".into()));
    }
    let file = pull_string(vm, "SAVE", 1)?;
    let db = open_world(&file)?;
    match family {
        Family::Aliases => {
            replace_aliases(&db, vm).map_err(|e| Error(format!("Aliases SAVE returns: {}", e.0)))
        }
        Family::Lambdas => {
            replace_lambdas(&db, vm).map_err(|e| Error(format!("Lambdas SAVE returns: {}", e.0)))
        }
        Family::Stacks => {
            replace_stacks(&db, vm).map_err(|e| Error(format!("Stacks SAVE returns: {}", e.0)))
        }
        Family::All => {
            // **`Aliases SAVE returns:` for all three** — the reference's own
            // paste, preserved.
            let wrong = |e: Error| Error(format!("Aliases SAVE returns: {}", e.0));
            replace_aliases(&db, vm).map_err(wrong)?;
            replace_lambdas(&db, vm).map_err(wrong)?;
            replace_stacks(&db, vm).map_err(wrong)?;
            prime_bootstrap(&db)
                .map_err(|e| Error(format!("Priming bootstrap returns: {}", e.0)))
        }
    }
}

/// `load` and its three families. `All` loads aliases, then lambdas, then
/// stacks, and **does not run bootstrap scripts** — that is `bootstrap`.
fn load_world(vm: &mut dyn Vm, family: Family) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for LOAD".into()));
    }
    let file = pull_string(vm, "LOAD", 1)?;
    let db = open_world(&file)?;
    load_families(vm, &db, family)
}

fn load_families(vm: &mut dyn Vm, db: &Database, family: Family) -> Result<(), Error> {
    match family {
        Family::Aliases => {
            load_aliases(db, vm).map_err(|e| Error(format!("Aliases LOAD returns: {}", e.0)))
        }
        Family::Lambdas => {
            load_lambdas(db, vm).map_err(|e| Error(format!("Lambdas LOAD returns: {}", e.0)))
        }
        Family::Stacks => {
            load_stacks(db, vm).map_err(|e| Error(format!("Stacks LOAD returns: {}", e.0)))
        }
        Family::All => {
            load_aliases(db, vm).map_err(|e| Error(format!("Aliases LOAD returns: {}", e.0)))?;
            load_lambdas(db, vm).map_err(|e| Error(format!("Lambdas LOAD returns: {}", e.0)))?;
            load_stacks(db, vm).map_err(|e| Error(format!("Stacks LOAD returns: {}", e.0)))
        }
    }
}

/// `bootstrap` — `load` and then every stored script, in name order.
///
/// **Its messages are `LOAD`'s, except the first.** The shallow-stack check
/// says `BOOTSTRAP`; the NO DATA and the casting failure both say `LOAD`,
/// because the reference pasted them from `stdlib_bund_load_base`.
fn bootstrap(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for BOOTSTRAP".into()));
    }
    let file = pull_string(vm, "LOAD", 1)?;
    let db = open_world(&file)?;
    load_families(vm, &db, Family::All).map_err(|e| Error(format!("LOAD returns: {}", e.0)))?;
    let scripts = read_all_bootstrap(&db)
        .map_err(|e| Error(format!("BOOTSTRAP discovery scripts returns: {}", e.0)))?;
    for script in scripts {
        // **Through `eval_source`, which is `bund_compile_and_eval`'s
        // equivalent** — the path `bund.eval` and `use` take.
        //
        // An earlier version compiled the script and handed it to
        // `eval_lambda` as a body. That left an `Exit` value on the stack:
        // `compile` ends a stream with the EXIT marker, which `eval_source`
        // breaks on and a lambda body does not, so the marker was applied as
        // a value. Caught by diffing `bootstrap` against the oracle.
        crate::singles::eval_source(vm, &script)
            .map_err(|e| Error(format!("BOOTSTRAP execution of script is failed: {}", e.0)))?;
    }
    Ok(())
}

/// `save.script` — store a snippet under a name in a world's bootstrap table.
///
/// **Three operands: the file on top, then the snippet, then the name**, so
/// the source reads `<name> <snippet> <file> save.script`.
///
/// **Its messages say `SAVE.BOOTSTRAP`, not `SAVE.SCRIPT`**, and its three
/// casting failures number the operands in an order that does not match the
/// order they are pulled — `#1` is the file, pulled first, but it is checked
/// last. Preserved.
///
/// **It fails on a world no `save` has primed — F148.** The reference deletes
/// any previous entry before inserting, and the table it deletes from is
/// created only by `save`.
fn save_script(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for SAVE.BOOTSTRAP".into()));
    }
    let file = pull_string3(vm, "SAVE", "SAVE.BOOTSTRAP", 1)?;
    let snippet = pull_string3(vm, "SAVE", "SAVE.BOOTSTRAP", 2)?;
    let name = pull_string3(vm, "SAVE", "SAVE.BOOTSTRAP", 3)?;
    let db = open_world(&file)?;
    // F148's first half: the table must already exist, and only `save` makes
    // one. redb cannot distinguish "absent" from "empty" on a write, so the
    // read is what refuses.
    let primed = {
        let tx = db
            .begin_read()
            .map_err(|e| Error(format!("Bootstrap SAVE.SCRIPT returns: {e}")))?;
        !matches!(tx.open_table(BOOTSTRAP), Err(TableError::TableDoesNotExist(_)))
    };
    if !primed {
        return Err(Error(
            "Bootstrap SAVE.SCRIPT returns: SAVE_BOOTSTRAP returns: Deleting previous bootstrap \
             entry returns: no such table: BOOTSTRAP"
                .into(),
        ));
    }
    let tx = db
        .begin_write()
        .map_err(|e| Error(format!("Bootstrap SAVE.SCRIPT returns: {e}")))?;
    {
        let mut t = tx
            .open_table(BOOTSTRAP)
            .map_err(|e| Error(format!("Bootstrap SAVE.SCRIPT returns: {e}")))?;
        t.insert(name.as_str(), snippet.as_str())
            .map_err(|e| Error(format!("Bootstrap SAVE.SCRIPT returns: {e}")))?;
    }
    tx.commit()
        .map_err(|e| Error(format!("Bootstrap SAVE.SCRIPT returns: {e}")))
}

/// `load.script` — **push a script's text; it is not evaluated.**
///
/// Two operands, the file on top and the name beneath.
fn load_script(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for LOAD.SCRIPT".into()));
    }
    let file = pull_string3(vm, "LOAD.SCRIPT", "LOAD.SCRIPT", 1)?;
    let name = pull_string3(vm, "LOAD.SCRIPT", "LOAD.SCRIPT", 3)?;
    let db = open_world(&file)?;
    let found = {
        let tx = db
            .begin_read()
            .map_err(|e| Error(format!("Error performing SCRIPT select: {e}")))?;
        match tx.open_table(BOOTSTRAP) {
            Ok(t) => t
                .get(name.as_str())
                .map_err(|e| Error(format!("Error performing SCRIPT select: {e}")))?
                .map(|v| v.value().to_string()),
            Err(TableError::TableDoesNotExist(_)) => None,
            Err(e) => return Err(Error(format!("Error compiling BOOTSTRAP select: {e}"))),
        }
    };
    match found {
        Some(script) => {
            vm.push(bund2_value::BundValue::str(script));
            Ok(())
        }
        // Wrapped twice, as the reference wraps it.
        None => Err(Error(format!(
            "Bootstrap LOAD.SCRIPT returns: Error getting script {name}: BOOTSTRAP discovery did \
             not find the script: {name}"
        ))),
    }
}

/// One operand for the three-operand words, whose NO DATA prefix and casting
/// prefix differ from each other.
fn pull_string3(
    vm: &mut dyn Vm,
    nodata: &str,
    casting: &str,
    n: u8,
) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{nodata} returns NO DATA #{n}")))?;
    v.as_str().ok_or_else(|| {
        Error(format!(
            "{casting} casting string #{n} returns: This Dynamic type is not string"
        ))
    })
}

/// `alias=` — the target an alias points at.
///
/// **Three spellings for one word, all preserved**: the shallow-stack message
/// says `?ALIAS`, the NO DATA and casting messages say `?ALIAS.GET`, and the
/// lookup failure says `ALIAS.GET`.
fn alias_get(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for ?ALIAS".into()));
    }
    let name = pull_string(vm, "?ALIAS.GET", 1)?;
    let target = vm
        .alias_pairs()
        .into_iter()
        .find(|(a, _)| *a == name)
        .map(|(_, t)| t)
        .ok_or_else(|| {
            Error(format!(
                "ALIAS.GET returned: VM Alias {name} not registered"
            ))
        })?;
    vm.push(bund2_value::BundValue::str(target));
    Ok(())
}
