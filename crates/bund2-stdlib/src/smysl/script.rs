//! `bund2-script`, the scripted provider (RFC-0014 §P8, R38): a provider of
//! a kind no SMYSL mapper has, which Bund2's registry builder makes itself
//! (§P3), and which answers from a script the crate's own tests arm.
//!
//! Compiled in only under `smysl-model-probe`, which no shipped build
//! enables, as `smysl-panic-probe` is (§S9).
//!
//! A script is armed for one endpoint, the provider entry's `endpoint`, so a
//! test that arms one does not reach another test's provider. The endpoint
//! is also what makes the provider local or not, as SMYSL's mappers decide
//! it (`ProviderConfig::is_local`): `http://localhost/<name>` is local,
//! `https://<host>/<name>` is not. Nothing is ever sent to it.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use smysl_provider::config::ProviderConfig;
use smysl_provider::{
    Capabilities, Completion, Probe, Provider, ProviderError, ProviderId, Request, Usage,
};

/// The provider kind a configuration names to reach the script.
pub const KIND: &str = "bund2-script";

/// One answer a script gives a `complete` call.
#[derive(Debug, Clone)]
pub enum Answer {
    /// The model's text.
    Text(String),
    /// A failed call.
    Fail(ProviderError),
    /// A panic on the calling thread.
    Panic,
    /// Create this file, then wait until it is removed, then answer the
    /// next queued answer.
    Pause(PathBuf),
}

/// What a probe answers.
#[derive(Debug, Clone)]
pub enum Probing {
    /// Reachable, offering these models.
    Models(Vec<String>),
    /// Answered, but unreachable.
    Unreachable,
    /// The probe itself failed.
    Fail(ProviderError),
}

/// One call the script received: the system text and every message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub system: String,
    pub messages: Vec<String>,
    pub temperature: String,
}

/// An action a test runs on the script's n-th `complete` call, before it
/// answers.
pub type Action = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct State {
    answers: VecDeque<Answer>,
    /// `None` is the configured model alone.
    probing: Option<Probing>,
    calls: Vec<Call>,
    probes: u64,
    actions: BTreeMap<u64, Action>,
}

static SCRIPTS: Mutex<BTreeMap<String, State>> = Mutex::new(BTreeMap::new());

fn with<T>(endpoint: &str, f: impl FnOnce(&mut State) -> T) -> T {
    let mut all = SCRIPTS.lock().unwrap_or_else(PoisonError::into_inner);
    f(all.entry(endpoint.to_string()).or_default())
}

/// Arm the script for `endpoint` with these answers, in order, replacing
/// whatever it held.
pub fn arm(endpoint: &str, answers: impl IntoIterator<Item = Answer>) {
    with(endpoint, |s| {
        *s = State {
            answers: answers.into_iter().collect(),
            ..State::default()
        };
    });
}

/// Queue more answers after those already armed.
pub fn queue(endpoint: &str, answers: impl IntoIterator<Item = Answer>) {
    with(endpoint, |s| s.answers.extend(answers));
}

/// What the script's probe answers.
pub fn probing(endpoint: &str, p: Probing) {
    with(endpoint, |s| s.probing = Some(p));
}

/// Run `action` on the script's `n`-th `complete` call, counting from 1.
pub fn on_call(endpoint: &str, n: u64, action: Action) {
    with(endpoint, |s| {
        s.actions.insert(n, action);
    });
}

/// The `complete` calls the script received, in order.
pub fn calls(endpoint: &str) -> Vec<Call> {
    with(endpoint, |s| s.calls.clone())
}

/// How many times it was probed.
pub fn probes(endpoint: &str) -> u64 {
    with(endpoint, |s| s.probes)
}

/// Forget the script for `endpoint`.
pub fn disarm(endpoint: &str) {
    SCRIPTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(endpoint);
}

/// The provider a `bund2-script` entry builds.
pub struct Script {
    cfg: ProviderConfig,
}

impl Script {
    pub fn new(cfg: ProviderConfig) -> Script {
        Script { cfg }
    }

    fn caps_of(cfg: &ProviderConfig) -> Capabilities {
        let mut caps = Capabilities::default();
        caps.model = cfg.model.clone();
        caps.context_window = cfg.context_window;
        caps.max_output = cfg.max_output;
        caps.structured = cfg.structured;
        caps.offline = cfg.is_local();
        caps
    }
}

impl Provider for Script {
    fn id(&self) -> ProviderId {
        self.cfg.id.clone()
    }

    fn caps(&self) -> Capabilities {
        Self::caps_of(&self.cfg)
    }

    #[expect(
        clippy::panic,
        reason = "RFC-0014 §P8: the script's deliberate panic, and a pause never released, under smysl-model-probe only"
    )]
    fn complete(&self, req: &Request) -> Result<Completion, ProviderError> {
        let endpoint = self.cfg.endpoint.as_str();
        let (n, action) = with(endpoint, |s| {
            s.calls.push(Call {
                system: req.system.clone(),
                messages: req.messages.iter().map(|m| m.content.clone()).collect(),
                temperature: req.temperature.to_string(),
            });
            let n = s.calls.len() as u64;
            (n, s.actions.remove(&n))
        });
        if let Some(act) = action {
            act();
        }
        loop {
            match with(endpoint, |s| s.answers.pop_front()) {
                Some(Answer::Text(text)) => {
                    let usage = Usage::estimated(
                        smysl_core::tokens(&req.system) as u64,
                        smysl_core::tokens(&text) as u64,
                    );
                    return Ok(Completion::new(text, self.cfg.model.clone(), usage));
                }
                Some(Answer::Fail(e)) => return Err(e),
                Some(Answer::Panic) => {
                    panic!("bund2-script: the armed panic on call {n} (RFC-0014 §P8)")
                }
                Some(Answer::Pause(flag)) => {
                    let _ = std::fs::write(&flag, b"arrived");
                    let start = Instant::now();
                    while flag.exists() {
                        if start.elapsed() > Duration::from_secs(30) {
                            panic!("bund2-script: the pause was not released within 30 s");
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                None => {
                    return Err(ProviderError::Config(format!(
                        "bund2-script at {endpoint} has no answer for call {n}"
                    )));
                }
            }
        }
    }

    fn probe(&self) -> Result<Probe, ProviderError> {
        let probing = with(&self.cfg.endpoint, |s| {
            s.probes += 1;
            s.probing.clone()
        });
        match probing {
            None => Ok(Probe::reachable(
                vec![self.cfg.model.clone()],
                self.caps(),
                "bund2-script",
            )),
            Some(Probing::Models(models)) => {
                Ok(Probe::reachable(models, self.caps(), "bund2-script"))
            }
            Some(Probing::Unreachable) => Ok(Probe::unreachable("bund2-script: unreachable")),
            Some(Probing::Fail(e)) => Err(e),
        }
    }
}
