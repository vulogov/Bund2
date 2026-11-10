//! The model layer's setup (RFC-0014 Phase 2, §P3): the configuration, found
//! and read as SMYSL's commands find and read it; the prompt override it
//! names; and the registry, built as SMYSL's `load_registry_at` builds it,
//! with each provider wrapped in Bund2's counting adaptor (§P6, R41).
//!
//! Compiled only under `smysl-model`. The words that use it are M12's to
//! M14's; this module answers what they need before any of them calls a
//! model, and calls none itself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use smysl_ingest::prompt::PromptOverride;
use smysl_provider::config::ProviderConfig;
use smysl_provider::{
    Capabilities, Completion, Config, Probe, Provider, ProviderError, ProviderId, Registry,
    Request, StreamMsg, TokenCount, Usage,
};

use super::{Code, Refusal};

/// SM13 `the model refused it: <message>`, the message as SMYSL's `Display`
/// writes it (§P9).
pub fn sm13(message: impl std::fmt::Display) -> Refusal {
    Refusal::new(Code::Sm13, format!("the model refused it: {message}"))
}

/// SM13 for a `ProviderError`, with its code where `ProviderError::code`
/// gives one (§P9).
pub fn refused(e: &ProviderError) -> Refusal {
    match e.code() {
        Some(code) => sm13(format!("{} {e}", code.as_str())),
        None => sm13(e),
    }
}

/// A configuration, and the directory a relative path written inside it
/// resolves against: the project for the sidecar, the file's own directory
/// for one `opts.config` names (§P3).
#[derive(Debug, Clone)]
pub struct Setup {
    pub config: Config,
    pub base: PathBuf,
}

/// The directory a library's sidecar lives in: the one that holds its root,
/// as SMYSL takes the store's parent (§P3). A root with no parent, `/`, has
/// its sidecar beside it.
fn project_of(root: &Path) -> PathBuf {
    root.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(root)
        .to_path_buf()
}

/// The configuration for a library at `root`, canonical, or the file `named`
/// (`opts.config`) names.
///
/// - A `named` file must be absolute (SM03), and present (SM13), as SMYSL
///   treats a `-C` file it cannot read as a usage error.
/// - Otherwise `.smysl/config.hjson` in the directory that holds `root`;
///   when that is absent, `Config::local_default()`.
/// - `Config::load`'s refusal is SM13, with the file's path, as SMYSL's
///   `load_config_from` writes it.
pub fn configuration(root: &Path, named: Option<&Path>) -> Result<Setup, Refusal> {
    let (path, required, base) = match named {
        Some(p) if !p.is_absolute() => {
            return Err(super::sm03(format!(
                "config {} is not an absolute path",
                p.display()
            )));
        }
        Some(p) => (p.to_path_buf(), true, project_of(p)),
        None => {
            let project = project_of(root);
            (project.join(Config::PATH), false, project)
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(src) => Config::load(&src)
            .map(|config| Setup { config, base })
            .map_err(|e| sm13(format!("{}: {e}", path.display()))),
        Err(e) if required => Err(sm13(format!("{}: {e}", path.display()))),
        Err(_) => Ok(Setup {
            config: Config::local_default(),
            base,
        }),
    }
}

/// The prompt override `ingest.prompt` names, read relative to the
/// configuration's base, or none. A missing or invalid override, or a
/// `*_file` it names that cannot be read, is SM13, as SMYSL's `load_prompt`
/// refuses it (§P3).
pub fn prompt(setup: &Setup) -> Result<Option<PromptOverride>, Refusal> {
    match &setup.config.ingest_prompt {
        None => Ok(None),
        Some(p) => PromptOverride::load_file(&setup.base.join(p))
            .map(Some)
            .map_err(sm13),
    }
}

/// The calls each provider answered with a `ProviderError`, by the error's
/// kind (§P6, R41). Shared by every adaptor one registry holds.
#[derive(Debug, Clone, Default)]
pub struct Failures(Arc<Mutex<BTreeMap<&'static str, u64>>>);

impl Failures {
    fn count(&self, e: &ProviderError) {
        let mut m = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *m.entry(kind(e)).or_insert(0) += 1;
    }

    /// Every failed call, of any kind.
    pub fn total(&self) -> u64 {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .sum()
    }

    /// The failed calls by kind.
    pub fn by_kind(&self) -> BTreeMap<&'static str, u64> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

/// A `ProviderError`'s kind, by its variant's name.
fn kind(e: &ProviderError) -> &'static str {
    match e {
        ProviderError::Unreachable => "unreachable",
        ProviderError::Unauthorized => "unauthorized",
        ProviderError::RateLimited { .. } => "rate-limited",
        ProviderError::ContextExceeded { .. } => "context-exceeded",
        ProviderError::StructuredUnsupported => "structured-unsupported",
        ProviderError::OfflineViolation => "offline-violation",
        ProviderError::Malformed(_) => "malformed",
        ProviderError::Upstream(..) => "upstream",
        ProviderError::Config(_) => "config",
        ProviderError::Truncated { .. } => "truncated",
        _ => "other",
    }
}

/// Bund2's adaptor: a provider that delegates every method and counts the
/// calls that failed (§P3, R41). A probe's failure is not counted: it is the
/// word's refusal, before any content is sent (§P6).
struct Counted {
    inner: Box<dyn Provider>,
    failures: Failures,
}

impl Counted {
    fn counted<T>(&self, r: Result<T, ProviderError>) -> Result<T, ProviderError> {
        if let Err(e) = &r {
            self.failures.count(e);
        }
        r
    }
}

impl Provider for Counted {
    fn id(&self) -> ProviderId {
        self.inner.id()
    }

    fn caps(&self) -> Capabilities {
        self.inner.caps()
    }

    fn complete(&self, req: &Request) -> Result<Completion, ProviderError> {
        self.counted(self.inner.complete(req))
    }

    fn stream(
        &self,
        req: &Request,
        tx: std::sync::mpsc::Sender<StreamMsg>,
    ) -> Result<Usage, ProviderError> {
        self.counted(self.inner.stream(req, tx))
    }

    fn count_tokens(&self, text: &str) -> TokenCount {
        self.inner.count_tokens(text)
    }

    fn probe(&self) -> Result<Probe, ProviderError> {
        self.inner.probe()
    }
}

/// A registry, the adaptors' shared count, and a `Warning` for each
/// provider this build cannot drive.
pub struct Built {
    pub registry: Registry,
    pub failures: Failures,
    pub warnings: Vec<String>,
}

/// Build one configured provider: `bund2-script` by Bund2 under
/// `smysl-model-probe` (§P8), every other kind by SMYSL's `map::build`.
fn provider(p: &ProviderConfig) -> Result<Box<dyn Provider>, ProviderError> {
    #[cfg(feature = "smysl-model-probe")]
    if p.kind == super::script::KIND {
        return Ok(Box::new(super::script::Script::new(p.clone())));
    }
    smysl_provider::map::build(p)
}

/// The registry for `config`, as SMYSL's `load_registry_at` builds it: each
/// provider, wrapped in the adaptor; the routes; the fallback chain; and
/// `offline`. A provider this build cannot drive is skipped, with a
/// `Warning` naming it, where SMYSL prints a line (§P3).
pub fn registry(config: &Config, offline: bool) -> Built {
    let failures = Failures::default();
    let mut warnings = Vec::new();
    let mut r = Registry::new().offline(offline);
    for p in config.providers.values() {
        match provider(p) {
            Ok(inner) => {
                r = r.with_provider(Box::new(Counted {
                    inner,
                    failures: failures.clone(),
                }));
            }
            Err(e) => warnings.push(format!("provider {}: {e}", p.id)),
        }
    }
    for (task, id) in &config.routing {
        r = r.route(*task, id.clone());
    }
    Built {
        registry: r.with_fallback(config.fallback.clone()),
        failures,
        warnings,
    }
}
