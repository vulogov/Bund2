//! Several Bund VMs, each on its own thread, joined by the local bus.
//!
//! The host here only starts VMs and waits for them. Everything they do is
//! Bund: the work, the channels, and the protocol between them, in the
//! `.bund` files beside this crate. A VM is not `Send`, so it never crosses
//! a thread; `bund2_async::host::Host` builds each one on its own thread,
//! and what crosses between them is the bus's encoded values (RFC-0007
//! §C8, D86).
//!
//!     threads_demo [workers]      workers: 1 to 16, 4 when not given
//!
//! Part 1 runs a pool of `workers` stages and a driver; part 2 runs a
//! pipeline of two stages and a driver. Each part ends when every one of
//! its VMs has, and a VM that fails ends the run with its error.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use bund2_api::{errln, sayln};
use bund2_async::host::{Host, Vm};
use bund2_runtime::Runtime;

const WORDS: &str = include_str!("../words.bund");
const STAGE: &str = include_str!("../stage.bund");
const POOL: &str = include_str!("../pool.bund");
const PIPELINE: &str = include_str!("../pipeline.bund");

/// What a VM thread answers: nothing, or why it stopped.
type Done = Result<(), String>;

/// One stage: where it reads, where it writes, and what it does between.
struct Stage {
    name: String,
    inbox: &'static str,
    outbox: &'static str,
    tally: &'static str,
    /// Bund run once before the stage starts, for state `step` keeps.
    setup: &'static str,
    /// The body of `step`: one value in, one value out.
    step: &'static str,
}

impl Stage {
    /// The program the stage's VM runs: its settings, the shared words,
    /// then `stage.bund`.
    fn program(&self) -> String {
        format!(
            ":name \"{}\" var\n:inbox \"{}\" var\n:outbox \"{}\" var\n:tally \"{}\" var\n{}\n\
             :step {{ {} }} register\n{WORDS}\n{STAGE}",
            self.name, self.inbox, self.outbox, self.tally, self.setup, self.step
        )
    }
}

fn spawn(host: &mut Host, label: String, program: String) -> Result<(String, Vm<Done>), String> {
    let vm = host.spawn(move |rt: &mut Runtime| rt.eval_str(&program).map_err(|e| e.0))?;
    Ok((label, vm))
}

/// Wait for every VM of a part, joining each as it finishes.
///
/// **A failed VM ends the wait at once.** The others may be waiting on a
/// channel it will never write, and `recv` does not block, so they would
/// spin for ever. Returning from `main` ends the process, and them with it.
fn settle(mut vms: Vec<(String, Vm<Done>)>) -> Result<(), String> {
    while !vms.is_empty() {
        let mut waiting = Vec::with_capacity(vms.len());
        for (label, vm) in vms {
            if !vm.finished() {
                waiting.push((label, vm));
                continue;
            }
            match vm.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) | Err(e) => return Err(format!("{label}: {e}")),
            }
        }
        vms = waiting;
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn pool(host: &mut Host, workers: usize) -> Result<(), String> {
    sayln!(
        "Part 1: a pool of {workers} worker VMs and a driver VM, on {} threads",
        workers + 1
    );
    sayln!();
    let started = Instant::now();
    let mut vms = Vec::with_capacity(workers + 1);
    for i in 1..=workers {
        let stage = Stage {
            name: format!("w{i}"),
            inbox: "jobs",
            outbox: "results",
            tally: "pool.tally",
            setup: "",
            step: "fib-job",
        };
        vms.push(spawn(host, stage.name.clone(), stage.program())?);
    }
    let driver = format!(":workers {workers} var\n{WORDS}\n{POOL}");
    vms.push(spawn(host, "the pool's driver".into(), driver)?);
    settle(vms)?;
    sayln!("({} ms)", started.elapsed().as_millis());
    Ok(())
}

fn pipeline(host: &mut Host) -> Result<(), String> {
    sayln!();
    sayln!("Part 2: a pipeline of two stage VMs and a driver VM, on 3 threads");
    sayln!();
    let stages = [
        Stage {
            name: "square".into(),
            inbox: "p.numbers",
            outbox: "p.squares",
            tally: "p.tally",
            setup: "",
            step: "square",
        },
        Stage {
            name: "sum".into(),
            inbox: "p.squares",
            outbox: "p.sums",
            tally: "p.tally",
            setup: ":total 0 var",
            step: "running-sum",
        },
    ];
    let mut vms = Vec::with_capacity(stages.len() + 1);
    for stage in &stages {
        vms.push(spawn(host, stage.name.clone(), stage.program())?);
    }
    vms.push(spawn(
        host,
        "the pipeline's driver".into(),
        format!("{WORDS}\n{PIPELINE}"),
    )?);
    settle(vms)
}

fn workers() -> Result<usize, String> {
    let Some(arg) = std::env::args().nth(1) else {
        return Ok(4);
    };
    match arg.parse::<usize>() {
        Ok(n) if (1..=16).contains(&n) => Ok(n),
        _ => Err(format!(
            "workers must be a number from 1 to 16, not {arg:?}"
        )),
    }
}

fn run() -> Result<(), String> {
    let workers = workers()?;
    // The pool's VMs have all finished before the pipeline's start, and the
    // host counts the VMs that exist at once.
    let mut host = Host::with_limit((workers + 1).max(3))?;
    pool(&mut host, workers)?;
    pipeline(&mut host)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            errln!("threads_demo: {e}");
            ExitCode::FAILURE
        }
    }
}
