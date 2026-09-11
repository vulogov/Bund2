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
