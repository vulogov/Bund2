//! The random words: ids, random integers, random strings, and `generator`.
//!
//! Every crate is the reference's, at the version its lock resolves, so a
//! value has the shape and the distribution the reference's has. Only the
//! values differ from run to run. What a golden can pin is therefore the
//! shape (a uuid is 36 characters) and the four `generator` types that are not
//! random at all: `sawtooth`, `periodic`, `sinusoidal` and `square` are fixed
//! sequences (`tests/probes/random-words.bund`).

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use rand::distributions::Distribution;
use rand_core::{RngCore, SeedableRng};

use crate::host::guard;
use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `id.uuid` — a random (v4) UUID as a string
/// (`reference/Bund/src/stdlib/functions/string/any_id.rs:13-16`).
fn id_uuid(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::str(uuid::Uuid::new_v4().to_string()));
    Ok(())
}

/// `id.ulid` — a ULID as a string (`any_id.rs:19-23`).
fn id_ulid(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::str(ulid::Ulid::new().to_string()));
    Ok(())
}

/// The Mersenne Twister behind `math.random.int`, seeded once from `fastrand`
/// with a number in `1..1000000000000`
/// (`reference/Bund/src/stdlib/functions/math/rand.rs:14-19`).
static MT: Mutex<Option<rand_mt::Mt64>> = Mutex::new(None);

/// The ChaCha20 generator behind `math.securerandom.int`, seeded once from
/// the operating system (`rand.rs:21-26`).
static CHACHA: Mutex<Option<rand_chacha::ChaCha20Rng>> = Mutex::new(None);

fn poisoned(what: &str) -> Error {
    Error::internal(format!(
        "the lock around {what} was poisoned by a panic in another native"
    ))
}

/// `math.random.int` and `math.securerandom.int` — a non-negative INTEGER
/// (`rand.rs:29-44`).
///
/// The draw is a `u64` taken `as i64` and then `.abs()`ed. The reference is
/// built in release, where `i64::MIN.abs()` wraps to itself, so one draw in
/// 2^64 is negative. `wrapping_abs` gives the same answer without the
/// overflow check a debug build would trip.
fn math_random_int(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut g = MT.lock().map_err(|_| poisoned("math.random.int"))?;
    let rng = g.get_or_insert_with(|| rand_mt::Mt64::new(fastrand::u64(1..1_000_000_000_000)));
    let v = rng.next_u64();
    drop(g);
    vm.push(BundValue::int((v as i64).wrapping_abs()));
    Ok(())
}

fn math_securerandom_int(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut g = CHACHA.lock().map_err(|_| poisoned("math.securerandom.int"))?;
    let rng = g.get_or_insert_with(rand_chacha::ChaCha20Rng::from_os_rng);
    let v = rng.next_u64();
    drop(g);
    vm.push(BundValue::int((v as i64).wrapping_abs()));
    Ok(())
}

/// Which `string.random.*` string to make
/// (`reference/Bund/src/stdlib/functions/string/random.rs:11-20`).
#[derive(Clone, Copy)]
enum Fake {
    Name,
    LastName,
    FullName,
    Password,
    Phone,
    IPv4,
    Word,
}

/// `string.random.*` — a random name, password, phone number, IPv4 address or
/// word (`random.rs:72-88`). No operand; the answer goes to the word's side.
fn fake(vm: &mut dyn Vm, side: Side, what: Fake) -> Result<(), Error> {
    let s = match what {
        Fake::Name => fakeit::name::first(),
        Fake::LastName => fakeit::name::last(),
        Fake::FullName => fakeit::name::full(),
        Fake::Password => password()?,
        Fake::Phone => fakeit::contact::phone_formatted(),
        Fake::IPv4 => fakeit::internet::ipv4_address(),
        Fake::Word => fakeit::words::word(),
    };
    side.push(vm, BundValue::str(s));
    Ok(())
}

/// Sixteen characters of every class, similar-looking ones excluded, at least
/// one of each class (`random.rs:23-35`). The reference `unwrap`s the result;
/// with this configuration the generator cannot refuse.
fn password() -> Result<String, Error> {
    passwords::PasswordGenerator {
        length: 16,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    }
    .generate_one()
    .map_err(|e| Error(format!("STRING.RANDOM.PASSWORD returns: {e}")))
}

/// `string.random.lorem` — `n` words of lorem ipsum (`random.rs:38-69`).
///
/// The count is cast `as usize` (`:60`), so the reference takes a negative
/// count as nearly `usize::MAX` words and runs out of memory. Bund2 refuses it
/// instead (F106).
fn lorem(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let n = v.as_int().ok_or_else(|| {
        Error(format!("Error casting in {prefix}: This Dynamic type is not integer"))
    })?;
    let Ok(n) = usize::try_from(n) else {
        return Err(Error(format!(
            "Error casting in {prefix}: a word count cannot be negative, got {n}"
        )));
    };
    let s = lipsum::lipsum_words_with_rng(rand::thread_rng(), n);
    side.push(vm, BundValue::str(s));
    Ok(())
}

/// A named generator (`reference/Bund/src/stdlib/functions/generators/mod.rs:21-49`).
enum Gen {
    Normal(statrs::distribution::Normal),
    Uniform(statrs::distribution::Uniform),
    LogNormal(statrs::distribution::LogNormal),
    Sawtooth(statrs::generate::InfiniteSawtooth),
    Periodic(statrs::generate::InfinitePeriodic),
    Sinusoidal(statrs::generate::InfiniteSinusoidal),
    Square(statrs::generate::InfiniteSquare),
}

struct Entry {
    g: Gen,
    /// How far into a sequence the next sample starts. Unused by the three
    /// distributions.
    skip: i64,
}

/// Every generator a program has made, by name. The reference keeps them in a
/// process-wide map (`generators/mod.rs:51-56`), and a generator made by one
/// program is still there for the next word, so this does too.
static GENERATORS: Mutex<BTreeMap<String, Entry>> = Mutex::new(BTreeMap::new());

fn generators() -> Result<MutexGuard<'static, BTreeMap<String, Entry>>, Error> {
    GENERATORS.lock().map_err(|_| poisoned("the generator table"))
}

/// A FLOAT parameter from the configuration, or its default when absent
/// (`reference/Bund/src/stdlib/helpers/conf.rs:5-22`).
///
/// The reference `cast_float().unwrap()`s the value (`generators/normal.rs:24`
/// and each sibling), so a parameter of the wrong kind crashes it. Bund2
/// reports it (F105).
fn float_param(conf: &BundValue, key: &str, default: f64) -> Result<f64, Error> {
    match conf.get(key) {
        None => Ok(default),
        Some(v) => match *v.unboxed() {
            BundValue::Float(f, _) => Ok(f),
            _ => Err(Error(format!(
                "GENERATOR: parameter {key} must be a FLOAT, not {}",
                v.dt()
            ))),
        },
    }
}

/// An INTEGER parameter, the same way (`generators/sawtooth.rs:23`).
fn int_param(conf: &BundValue, key: &str, default: i64) -> Result<i64, Error> {
    match conf.get(key) {
        None => Ok(default),
        Some(v) => v.as_int().ok_or_else(|| {
            Error(format!(
                "GENERATOR: parameter {key} must be an INTEGER, not {}",
                v.dt()
            ))
        }),
    }
}

/// `generator` — make a named generator from a configuration MAP
/// (`reference/Bund/src/stdlib/functions/generators/generator.rs:10-41`).
///
/// The configuration is on top and the name beneath it, so the source reads
/// `<name> <conf> generator`. Its `type` picks one of seven kinds; each reads
/// its own parameters with defaults, in the order the reference reads them,
/// which decides which bad parameter is reported first. A second `generator`
/// with the same name replaces the first.
fn generator(vm: &mut dyn Vm) -> Result<(), Error> {
    use statrs::distribution::{LogNormal, Normal, Uniform};
    use statrs::generate::{InfinitePeriodic, InfiniteSawtooth, InfiniteSinusoidal, InfiniteSquare};
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline GENERATOR".into()));
    }
    let conf = vm
        .pull()
        .ok_or_else(|| Error("GENERATOR returns: NO DATA #1".into()))?;
    let name_v = vm
        .pull()
        .ok_or_else(|| Error("GENERATOR returns: NO DATA #2".into()))?;
    let name = name_v.as_str().ok_or_else(|| {
        Error("GENERATOR name casting returns: This Dynamic type is not string".into())
    })?;
    let ty_v = conf.get("type").unwrap_or_else(|| BundValue::str("unknown"));
    // `cast_string().unwrap()` in the reference (`generator.rs:25`): F105.
    let ty = ty_v.as_str().ok_or_else(|| {
        Error(format!("GENERATOR: parameter type must be a string, not {}", ty_v.dt()))
    })?;
    // Each statrs distribution has its own error type; all display.
    fn dist_err(e: impl std::fmt::Display) -> Error {
        Error(format!("GENERATOR returned: {e}"))
    }
    let g = match ty.as_str() {
        // `generators/normal.rs:24-30`.
        "normal" => {
            let mean = float_param(&conf, "Mean", 0.0)?;
            let dev = float_param(&conf, "Deviation", 1.0)?;
            Gen::Normal(Normal::new(mean, dev).map_err(dist_err)?)
        }
        // `generators/uniform.rs:24-30`: Max is read before Min.
        "uniform" => {
            let max = float_param(&conf, "Max", 1.0)?;
            let min = float_param(&conf, "Min", 0.0)?;
            Gen::Uniform(Uniform::new(min, max).map_err(dist_err)?)
        }
        // `generators/lognormal.rs:24-30`.
        "lognormal" => {
            let loc = float_param(&conf, "Location", 0.0)?;
            let scale = float_param(&conf, "Scale", 1.0)?;
            Gen::LogNormal(LogNormal::new(loc, scale).map_err(dist_err)?)
        }
        // `generators/sawtooth.rs:23-28`.
        "sawtooth" => {
            let period = int_param(&conf, "Period", 10)?;
            let high = float_param(&conf, "High", 1.0)?;
            let low = float_param(&conf, "Low", 0.0)?;
            let delay = int_param(&conf, "Delay", 1)?;
            Gen::Sawtooth(InfiniteSawtooth::new(period, high, low, delay))
        }
        // `generators/periodic.rs:24-31`.
        "periodic" => {
            let sampling = float_param(&conf, "Sampling", 10.0)?;
            let freq = float_param(&conf, "Freq", 2.0)?;
            let amplitude = float_param(&conf, "Amplitude", 10.0)?;
            let phase = float_param(&conf, "Phase", 1.0)?;
            let delay = int_param(&conf, "Delay", 1)?;
            Gen::Periodic(InfinitePeriodic::new(sampling, freq, amplitude, phase, delay))
        }
        // `generators/sinusoidal.rs:24-32`: Mean is read after Phase.
        "sinusoidal" => {
            let sampling = float_param(&conf, "Sampling", 10.0)?;
            let freq = float_param(&conf, "Freq", 2.0)?;
            let amplitude = float_param(&conf, "Amplitude", 10.0)?;
            let phase = float_param(&conf, "Phase", 1.0)?;
            let mean = float_param(&conf, "Mean", 5.0)?;
            let delay = int_param(&conf, "Delay", 1)?;
            Gen::Sinusoidal(InfiniteSinusoidal::new(
                sampling, freq, amplitude, mean, phase, delay,
            ))
        }
        // `generators/square.rs:23-29`.
        "square" => {
            let high_d = int_param(&conf, "DurationHigh", 1)?;
            let low_d = int_param(&conf, "DurationLow", 1)?;
            let high = float_param(&conf, "High", 1.0)?;
            let low = float_param(&conf, "Low", 0.0)?;
            let delay = int_param(&conf, "Delay", 1)?;
            Gen::Square(InfiniteSquare::new(high_d, low_d, high, low, delay))
        }
        other => return Err(Error(format!("Unknown GENERATOR type: {other}"))),
    };
    generators()?.insert(name, Entry { g, skip: 0 });
    Ok(())
}

/// One value from a generator. A distribution draws from `thread_rng`
/// (`generators/normal.rs:49-50`). A sequence restarts from its beginning and
/// skips the values already taken (`generators/sawtooth.rs:47-51`), which is
/// how the reference steps a sequence it keeps by value.
///
/// The sequences are infinite, so the "Failed to sample" arm is not reached.
/// Its text is still the reference's, copy-paste included: `sinusoidal` says
/// `periodic` for a single sample (`generators/sinusoidal.rs:53`), and
/// `square` says `sawtooth` (`generators/square.rs:50,85`).
fn draw(e: &mut Entry, batch: bool) -> Result<f64, Error> {
    let skip = usize::try_from(e.skip).unwrap_or(0);
    let next = match &e.g {
        Gen::Normal(d) => return Ok(d.sample(&mut rand::thread_rng())),
        Gen::Uniform(d) => return Ok(d.sample(&mut rand::thread_rng())),
        Gen::LogNormal(d) => return Ok(d.sample(&mut rand::thread_rng())),
        // Each arm steps a copy, so the stored sequence stays at its start.
        Gen::Sawtooth(s) => { *s }.nth(skip).ok_or("sawtooth"),
        Gen::Periodic(s) => { *s }.nth(skip).ok_or("periodic"),
        Gen::Sinusoidal(s) => { *s }
            .nth(skip)
            .ok_or(if batch { "sinusoidal" } else { "periodic" }),
        Gen::Square(s) => { *s }.nth(skip).ok_or("sawtooth"),
    };
    match next {
        Ok(v) => {
            e.skip += 1;
            Ok(v)
        }
        Err(kind) => Err(Error(format!("Failed to sample next {kind} value"))),
    }
}

fn is_sequence(g: &Gen) -> bool {
    !matches!(g, Gen::Normal(_) | Gen::Uniform(_) | Gen::LogNormal(_))
}

/// `generator.sample` — one FLOAT from the named generator
/// (`reference/Bund/src/stdlib/functions/generators/generator.rs:44-106`).
fn generator_sample(vm: &mut dyn Vm) -> Result<(), Error> {
    const P: &str = "GENERATOR.SAMPLE";
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {P}")));
    }
    let name_v = vm.pull().ok_or_else(|| Error(format!("{P} returns: NO DATA #1")))?;
    let name = name_v.as_str().ok_or_else(|| {
        Error(format!("{P} casting a name returns: This Dynamic type is not string"))
    })?;
    let mut table = generators()?;
    let e = table
        .get_mut(&name)
        .ok_or_else(|| Error(format!("{P} not found generator: {name}")))?;
    let v = draw(e, false)?;
    drop(table);
    vm.push(BundValue::float(v));
    Ok(())
}

/// `generator.sample*` — a list of `n` FLOATs from the named generator
/// (`generator.rs:109-179`). The count is on top and the name beneath it.
///
/// **A sequence skips one extra value after each batch**: the reference steps
/// the position once per value and once more after the loop
/// (`generators/sawtooth.rs:86-89`, and the same in each sequence file). So
/// `3 generator.sample*` then `generator.sample` misses the fourth value and
/// answers the fifth. F107, reproduced. A negative count answers an empty
/// list.
fn generator_sample_n(vm: &mut dyn Vm) -> Result<(), Error> {
    const P: &str = "GENERATOR.SAMPLE*";
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {P}")));
    }
    let n_v = vm.pull().ok_or_else(|| Error(format!("{P} returns: NO DATA #2")))?;
    let n = n_v.as_int().ok_or_else(|| {
        Error(format!("{P} casting a name returns: This Dynamic type is not integer"))
    })?;
    let name_v = vm.pull().ok_or_else(|| Error(format!("{P} returns: NO DATA #1")))?;
    let name = name_v.as_str().ok_or_else(|| {
        Error(format!("{P} casting a name returns: This Dynamic type is not string"))
    })?;
    let mut table = generators()?;
    let e = table
        .get_mut(&name)
        .ok_or_else(|| Error(format!("{P} not found generator: {name}")))?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(BundValue::float(draw(e, true)?));
    }
    if is_sequence(&e.g) {
        e.skip += 1;
    }
    drop(table);
    vm.push(BundValue::list(out));
    Ok(())
}

pub fn register(r: &mut Registry) {
    // `reference/Bund/src/stdlib/functions/string/any_id.rs:35-36`.
    r.register_native("id.uuid", id_uuid, eff(0, 1), WordKind::Sync);
    r.register_native("id.ulid", id_ulid, eff(0, 1), WordKind::Sync);
    // `reference/Bund/src/stdlib/functions/math/rand.rs:60-61`.
    r.register_native("math.random.int", math_random_int, eff(0, 1), WordKind::Sync);
    r.register_native(
        "math.securerandom.int",
        math_securerandom_int,
        eff(0, 1),
        WordKind::Sync,
    );
    // `reference/Bund/src/stdlib/functions/string/random.rs:155-170`.
    macro_rules! fake_pair {
        ($name:literal, $dot:literal, $what:expr) => {
            r.register_native($name, |vm| fake(vm, Side::Stack, $what), eff(0, 1), WordKind::Sync);
            r.register_native($dot, |vm| fake(vm, Side::Bench, $what), eff(0, 0), WordKind::Sync);
        };
    }
    fake_pair!("string.random.name", "string.random.name.", Fake::Name);
    fake_pair!("string.random.lastname", "string.random.lastname.", Fake::LastName);
    fake_pair!("string.random.fullname", "string.random.fullname.", Fake::FullName);
    fake_pair!("string.random.password", "string.random.password.", Fake::Password);
    fake_pair!("string.random.phone", "string.random.phone.", Fake::Phone);
    fake_pair!("string.random.ipv4", "string.random.ipv4.", Fake::IPv4);
    fake_pair!("string.random.word", "string.random.word.", Fake::Word);
    r.register_native(
        "string.random.lorem",
        |vm| lorem(vm, Side::Stack, "STRING.RANDOM.LOREM"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.random.lorem.",
        |vm| lorem(vm, Side::Bench, "STRING.RANDOM.LOREM."),
        eff(0, 0),
        WordKind::Sync,
    );
    // `reference/Bund/src/stdlib/functions/generators/mod.rs:68-70`.
    r.register_native("generator", generator, eff(2, 0), WordKind::Sync);
    r.register_native("generator.sample", generator_sample, eff(1, 1), WordKind::Sync);
    r.register_native("generator.sample*", generator_sample_n, eff(2, 1), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;

    fn run(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn top_text(src: &str) -> String {
        match run(src) {
            Ok(i) => i.peek().map(|v| v.display()).unwrap_or_default(),
            Err(e) => panic!("{src} failed: {e}"),
        }
    }

    #[test]
    fn ids_have_their_shapes() {
        assert_eq!(top_text("id.uuid").len(), 36);
        assert_eq!(top_text("id.ulid").len(), 26);
    }

    #[test]
    fn random_ints_are_not_negative_in_practice() {
        let i = run("math.random.int math.securerandom.int").expect("runs");
        assert_eq!(i.depth(), 2);
    }

    #[test]
    fn lorem_counts_words_and_refuses_a_negative_count() {
        assert_eq!(top_text("5 string.random.lorem").split_whitespace().count(), 5);
        let e = match run("-1 string.random.lorem") {
            Ok(_) => panic!("a negative count was accepted"),
            Err(e) => e,
        };
        assert!(e.contains("cannot be negative"), "{e}");
    }

    #[test]
    fn a_password_has_sixteen_characters() {
        assert_eq!(top_text("string.random.password").chars().count(), 16);
    }

    /// F105: the reference crashes on a parameter of the wrong kind.
    #[test]
    fn a_parameter_of_the_wrong_kind_is_reported() {
        let e = match run("\"t-bad\" dict \"type\" \"normal\" set \"Mean\" 1 set generator") {
            Ok(_) => panic!("an INTEGER Mean was accepted"),
            Err(e) => e,
        };
        assert!(e.contains("parameter Mean must be a FLOAT"), "{e}");
    }

    #[test]
    fn a_normal_generator_samples_floats() {
        let t = top_text(
            "\"t-normal\" dict \"type\" \"normal\" set generator \"t-normal\" 3 generator.sample*",
        );
        assert_eq!(t.matches(" :: ").count(), 3, "{t}");
    }
}
