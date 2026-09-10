//! `cargo xtask effects` — RFC-0004 criterion 2.
//!
//! Joins the effect **Bund2 declares** against the effect the oracle was
//! **observed** to have, and reports the disagreements. Both sides are asked
//! rather than inferred: `bund2 effects` dumps the registry, and
//! `docs/arity.md`'s probed column comes from running each word against the
//! real interpreter.
//!
//! # Why this is worth a tool
//!
//! F18's argument is that a static arity that lies does not stay cosmetic.
//! RFC-0004 infers Bund words' effects by composing these, and RFC-0005 orders
//! JIT guards by them, so a wrong number propagates into two later RFCs. And
//! nothing else checks them: an effect is written by hand at the registration
//! site, next to a function whose body may say something different, and no
//! test reads it.
//!
//! It found three on its first run, one of which was a plain bug — `?object`
//! was changed to peek and its declared effect was not updated with it.
//!
//! # Three buckets, not two
//!
//! A disagreement does **not** always mean Bund2 is wrong, and collapsing the
//! two cases would make this report as vacuous as the failure list
//! `conform --blocked-on` was written to replace.
//!
//! The probe measures **one arm** of a polymorphic word, because it feeds
//! sentinels of one type at a time. `apply` reads `1 -> 1` because an integer
//! sentinel is pushed back unchanged; applied to a CALL it is whatever the
//! call does. `graph!` reads `0 -> 1` because a non-LIST sentinel is pushed
//! back and an empty graph built; given two lists it consumes both. Neither
//! number is wrong about what was measured, and neither is the word's arity.
//!
//! So a divergence is either a defect or an **arm**, and which one it is
//! belongs in `tests/golden/EFFECTS.txt` with a reason — the same shape as
//! `DEVIATIONS.txt`, and for the same reason: an exclusion hides a regression,
//! a recorded reason does not.

use std::collections::BTreeMap;
use std::path::Path;

/// Where accepted divergences live.
const REGISTER: &str = "tests/golden/EFFECTS.txt";

struct Row {
    word: String,
    declared: (u32, u32),
    probed: (u32, u32),
}

fn bund2_effects(repo: &Path) -> Result<BTreeMap<String, (u32, u32)>, String> {
    let exe = crate::buildcli::bund2(repo, false, "")?;
    let out = std::process::Command::new(&exe)
        .arg("effects")
        .output()
        .map_err(|e| format!("running {}: {e}", exe.display()))?;
    let mut map = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut it = line.split('\t');
        let (Some(n), Some(c), Some(p)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        if let (Ok(c), Ok(p)) = (c.parse(), p.parse()) {
            map.insert(n.to_string(), (c, p));
        }
    }
    Ok(map)
}

/// The probed columns of `docs/arity.md`.
///
/// A `consumed` written `N+` is a **floor**, not an arity — the word consumed
/// everything it was given — so it is skipped rather than compared. Comparing
/// a declared count against a floor would report a disagreement that means
/// nothing.
fn probed_effects(repo: &Path) -> Result<BTreeMap<String, (u32, u32)>, String> {
    let path = repo.join("docs/arity.md");
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("reading {}: {e}. Run `cargo xtask arity` first.", path.display()))?;
    let mut map = BTreeMap::new();
    for line in body.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(word), Some(c), Some(p)) = (cells.get(1), cells.get(4), cells.get(5)) else {
            continue;
        };
        let word = word.trim_matches('`');
        if c.ends_with('+') {
            continue;
        }
        if let (Ok(c), Ok(p)) = (c.parse(), p.parse()) {
            map.insert(word.to_string(), (c, p));
        }
    }
    Ok(map)
}

/// `<word>\t<reason>` — divergences that are arms rather than defects.
fn accepted(repo: &Path) -> BTreeMap<String, String> {
    let Ok(body) = std::fs::read_to_string(repo.join(REGISTER)) else {
        return BTreeMap::new();
    };
    body.lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_once('\t'))
        .map(|(w, r)| (w.trim().to_string(), r.trim().to_string()))
        .collect()
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let declared = bund2_effects(&repo)?;
    let probed = probed_effects(&repo)?;
    let approved = accepted(&repo);

    let mut agree = 0usize;
    let mut diverge: Vec<Row> = Vec::new();
    let mut not_in_table: Vec<&String> = Vec::new();
    for (word, d) in &declared {
        match probed.get(word) {
            None => not_in_table.push(word),
            Some(p) if p == d => agree += 1,
            Some(p) => diverge.push(Row {
                word: word.clone(),
                declared: *d,
                probed: *p,
            }),
        }
    }
    let (arms, defects): (Vec<&Row>, Vec<&Row>) =
        diverge.iter().partition(|r| approved.contains_key(&r.word));

    println!("# cargo xtask effects\n");
    println!("The arity Bund2 **declares** against the arity the oracle was");
    println!("**observed** to have. RFC-0004 criterion 2.\n");
    println!("Nothing else checks a declared effect: it is written by hand at the");
    println!("registration site and no test reads it, while RFC-0004 composes");
    println!("them and RFC-0005 orders JIT guards by them. F18's point is that a");
    println!("static arity that lies does not stay cosmetic.\n");

    println!("  {:<34}{:>5}", "natives declaring an effect", declared.len());
    println!("  {:<34}{:>5}", "comparable against the table", agree + diverge.len());
    println!("  {:<34}{:>5}", "agree", agree);
    println!("  {:<34}{:>5}", "recorded as an arm", arms.len());
    println!("  {:<34}{:>5}", "DISAGREE", defects.len());
    println!("  {:<34}{:>5}", "not in the table", not_in_table.len());
    println!();

    if !defects.is_empty() {
        println!("## disagree\n");
        println!("  Each is a defect until recorded otherwise. A declared effect");
        println!("  that does not match what the word does is what F18 forbids.\n");
        for r in &defects {
            println!(
                "  {:<30} declared {}->{}   probed {}->{}",
                r.word, r.declared.0, r.declared.1, r.probed.0, r.probed.1
            );
        }
        println!();
    }

    if !arms.is_empty() {
        println!("## recorded as an arm\n");
        println!("  The probe feeds sentinels of one type, so for a polymorphic");
        println!("  word it measures one arm rather than the arity. These are");
        println!("  recorded in {REGISTER} with the reason.\n");
        for r in &arms {
            let why = approved.get(&r.word).map(String::as_str).unwrap_or("");
            println!(
                "  {:<30} declared {}->{}   probed {}->{}   {why}",
                r.word, r.declared.0, r.declared.1, r.probed.0, r.probed.1
            );
        }
        println!();
    }

    println!("## not in the table\n");
    println!("  {} native(s) Bund2 declares that the probe could not", not_in_table.len());
    println!("  measure — it refused every sentinel on type grounds, or the word");
    println!("  is unsafe to run. **Listed, not counted as agreement**: a word");
    println!("  the probe could not reach is evidence of nothing, and folding it");
    println!("  into a pass is how a criterion goes vacuous.\n");
    let names: Vec<&str> = not_in_table.iter().map(|s| s.as_str()).collect();
    for chunk in names.chunks(6) {
        println!("      {}", chunk.join("  "));
    }
    println!();

    if defects.is_empty() {
        println!("  RFC-0004 criterion 2: no declared effect disagrees with the");
        println!("  probed table.\n");
        Ok(())
    } else {
        Err(format!(
            "{} declared effect(s) disagree with docs/arity.md",
            defects.len()
        ))
    }
}
