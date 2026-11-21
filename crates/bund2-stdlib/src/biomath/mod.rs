//! Biomathematics and genetics: a family of Bund2's own (RFC-0015, D250).
//!
//! **None of this is the reference's.** No reference word bears these names,
//! so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero (D72), and what
//! checks the family is its unit tests rather than a golden (D21).
//!
//! The family is built in the RFC's milestones, in order. M1 lays what every
//! later one uses:
//!
//! - [`refuse`]: S11's refusals, E101 to E115, each with its text;
//! - [`rng`]: the generator a drawing word draws from, the VM's own through
//!   [`bund2_api::Vm::rng`] (§S8, D250 B4), and `rng.seed`'s reseeding.
//!
//! M2 adds the first words:
//!
//! - [`special`]: §S4's sixteen `math.` special functions.
//!
//! M3 adds §S5 and §S6's descriptive words, over [`operand`]'s readers:
//!
//! - [`vector`]: the `vec.` words;
//! - [`describe`]: `stat.var` to `stat.cor.kendall`.
//!
//! M4 adds §S7 and §S8's words:
//!
//! - [`dist`]: the distributions, as MAPs, and the words that ask them;
//! - [`rng`]: `rng.seed` and `rng.uniform`.
//!
//! M5 adds the rest of §S6:
//!
//! - [`infer`]: the `stat.test.` words, `stat.padjust.*` and `stat.lm`.
//!
//! M6 adds §S9:
//!
//! - [`linalg`]: the `linalg.` words, over `nalgebra`.
//!
//! M7 adds §S10:
//!
//! - [`sequence`]: the `bio.` sequence words, and NCBI's genetic codes.
//!
//! M8 adds §S12:
//!
//! - [`align`]: `bio.align` and `bio.align.score`, Gotoh's affine-gap
//!   alignment, global, semiglobal and local.
//!
//! M9 adds §S13:
//!
//! - [`popgen`]: the `popgen.` words, Hardy–Weinberg's exact test to
//!   Wright–Fisher trajectories.
//!
//! M10 adds §S14:
//!
//! - [`phylo`]: flat trees, distances, UPGMA and neighbour joining, Newick
//!   both ways, and a tree as a GRAPH.
//!
//! M11 adds §S15:
//!
//! - [`dynamics`]: `ode.rk4`, `ode.rk45` and `sim.gillespie`, whose models
//!   are lambdas run as RFC-0013's bridges run theirs.
//!
//! M12 adds §S16:
//!
//! - [`fit`]: `fit.nls`, Levenberg–Marquardt over a model lambda, and
//!   `fit.logistic`, logistic regression by IRLS.
//!
//! M13 adds no word: `bund_examples/biomath/` runs the RFC's worked
//! examples and a rule base over the family.
//!
//! **What M1 links.** `nalgebra` and `rand_distr` become direct dependencies
//! at the versions the lock already held, and `statrs` gains its `nalgebra`
//! feature, for the multivariate families (§S7). The lock gains no package.
//!
//! RFC-0016 adds statistical inference to the family, in its own milestones.
//! Its M1 adds §S4 and §S6:
//!
//! - [`infer`]: `stat.padjust.hochberg`, `.hommel` and `.by`;
//! - [`round`]: `math.round.places`, `math.signif`, `string.fixed` and
//!   `string.sci`.
//!
//! Its M2 and M3 add §S2's `.opts` words to [`infer`]: estimates,
//! intervals and one-sided p for the t tests, the binomial, Fisher's test,
//! Mann–Whitney and KS. Its M4 adds §S3's correlation tests, in
//! [`infer::cor`]. Its M5 adds the last two sections:
//!
//! - [`effect`]: §S5's effect sizes, `stat.effect.*`;
//! - [`sample`]: §S7's shuffling and sampling, `rng.shuffle` and
//!   `rng.choice*`.
//!
//! RFC-0017 adds frames, factors and missing values. Its M1 adds §S3 and
//! the first of §S4:
//!
//! - [`na`]: `na`, `na.is`, `na.count`, `na.omit` and `na.fill`;
//! - [`frame`]: §S2's frame and its check, and `frame.new`, `.check`,
//!   `.nrow`, `.ncol`, `.names`, `.column` and `.rows`.
//!
//! Its M2 adds the rest of §S4, in [`frame::reshape`]: `frame.with`,
//! `.select`, `.take`, `.head`, `.filter`, `.complete`, `.sort`, `.factor`
//! and `.factor.levels`. Its M3 adds §S6, in [`frame::csv`]:
//! `frame.csv.parse`, `frame.csv.text` and `frame.text`.
//!
//! RFC-0018 adds models over frames. Its M1 adds §S2:
//!
//! - [`model`]: `model.matrix`, the design and its check.

pub mod align;
pub mod describe;
pub mod dist;
pub mod dynamics;
pub mod effect;
pub mod fit;
pub mod frame;
pub mod infer;
pub mod linalg;
pub mod model;
pub mod na;
pub(crate) mod operand;
pub mod phylo;
pub mod popgen;
pub mod refuse;
pub mod rng;
pub mod round;
pub mod sample;
pub mod sequence;
pub mod special;
pub mod vector;

/// Register the family's words.
pub fn register(r: &mut bund2_api::Registry) {
    special::register(r);
    vector::register(r);
    describe::register(r);
    dist::register(r);
    infer::register(r);
    linalg::register(r);
    sequence::register(r);
    align::register(r);
    popgen::register(r);
    phylo::register(r);
    dynamics::register(r);
    fit::register(r);
    rng::register(r);
    round::register(r);
    effect::register(r);
    sample::register(r);
    na::register(r);
    frame::register(r);
    model::register(r);
}

#[cfg(test)]
mod testkit;
