//! RFC-0015 §S11's refusals, E101 to E115.
//!
//! Each is reported as `<WORD>: E1nn <text>`, the word upper-cased, as
//! RFC-0013's are, through the word's `Err` and so `Vm::report` (D36). The
//! text is S11's template filled in; a test below holds every one.
//!
//! **E110 and E111 split on who chose the work.** E110 is an iterative method
//! that stopped short of converging: a decomposition, a quantile's polish,
//! inverse iteration, IRLS, Levenberg–Marquardt, an ODE step underflowing.
//! E111 is a cap on work a program's operands choose: steps, events, cells,
//! generations, draws (D39).

use bund2_api::Error;

/// One of S11's refusals, with what its text names.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// E101: an operand outside the function's domain.
    Domain {
        operand: String,
        function: String,
        value: String,
    },
    /// E102: lengths or shapes that do not agree.
    Disagree { what: String, n: usize, m: usize },
    /// E103: a MATRIX or LIST with a cell that is not a number, or ragged rows.
    NotNumbers {
        operand: String,
        container: &'static str,
        cell: String,
    },
    /// E104: a distribution's parameters refused by `statrs`.
    Parameters { family: String, message: String },
    /// E105: an operation a family does not define.
    Undefined { family: String, operation: String },
    /// E106: a singular matrix.
    Singular,
    /// E106: a singular design, naming the first column that depends on
    /// those before it (RFC-0018 §S8). `column` is the name as rendered.
    SingularColumn { column: String },
    /// E107: a non-symmetric matrix where symmetry is required.
    NotSymmetric { i: usize, j: usize, by: f64 },
    /// E108: a complex eigenvalue where a real one is asked for.
    Complex { re: f64, im: f64 },
    /// E108: two eigenvalues of the largest modulus, so no dominant one.
    NotUnique { first: f64, second: f64 },
    /// E109: a character outside the sequence's alphabet.
    Alphabet {
        ch: char,
        pos: usize,
        alphabet: String,
    },
    /// E110: an iterative method that did not converge within its cap.
    NoConvergence { method: String, cap: u64 },
    /// E111: a cap on work reached (D39).
    Cap { what: String, cap: u64 },
    /// E112: a model lambda that left the wrong thing.
    Model { left: String, expected: String },
    /// E113: a malformed Newick string.
    Newick { pos: usize, why: String },
    /// E114: an answer that overflows a FLOAT from finite operands.
    Overflow { expression: String },
    /// E115: a drained series where a LIST or METRICS is required.
    Drained { operand: String },
}

impl Refusal {
    /// The code, `E101` to `E115`.
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::Domain { .. } => "E101",
            Refusal::Disagree { .. } => "E102",
            Refusal::NotNumbers { .. } => "E103",
            Refusal::Parameters { .. } => "E104",
            Refusal::Undefined { .. } => "E105",
            Refusal::Singular | Refusal::SingularColumn { .. } => "E106",
            Refusal::NotSymmetric { .. } => "E107",
            Refusal::Complex { .. } | Refusal::NotUnique { .. } => "E108",
            Refusal::Alphabet { .. } => "E109",
            Refusal::NoConvergence { .. } => "E110",
            Refusal::Cap { .. } => "E111",
            Refusal::Model { .. } => "E112",
            Refusal::Newick { .. } => "E113",
            Refusal::Overflow { .. } => "E114",
            Refusal::Drained { .. } => "E115",
        }
    }

    /// S11's template, filled in.
    pub fn text(&self) -> String {
        match self {
            Refusal::Domain {
                operand,
                function,
                value,
            } => format!("{operand} is outside {function}'s domain: {value}"),
            Refusal::Disagree { what, n, m } => format!("{what} do not agree: {n} and {m}"),
            Refusal::NotNumbers {
                operand,
                container,
                cell,
            } => format!("{operand} is not a {container} of numbers: {cell}"),
            Refusal::Parameters { family, message } => {
                format!("invalid {family} parameters: {message}")
            }
            Refusal::Undefined { family, operation } => format!("{family} has no {operation}"),
            Refusal::Singular => "the matrix is singular".into(),
            Refusal::SingularColumn { column } => {
                format!("the matrix is singular: column {column} depends on those before it")
            }
            Refusal::NotSymmetric { i, j, by } => {
                format!("the matrix is not symmetric: [{i},{j}] and [{j},{i}] differ by {by}")
            }
            Refusal::Complex { re, im } => {
                format!("an eigenvalue is complex: {re} ± {}i", im.abs())
            }
            Refusal::NotUnique { first, second } => {
                format!("the dominant eigenvalue is not unique: {first}, {second}")
            }
            Refusal::Alphabet { ch, pos, alphabet } => {
                format!("{ch} at {pos} is not in the {alphabet} alphabet")
            }
            Refusal::NoConvergence { method, cap } => {
                format!("{method} did not converge within {cap} iterations")
            }
            Refusal::Cap { what, cap } => format!("{what} exceeds its cap of {cap}"),
            Refusal::Model { left, expected } => {
                format!("the model lambda left {left}, where {expected} was required")
            }
            Refusal::Newick { pos, why } => format!("malformed Newick at {pos}: {why}"),
            Refusal::Overflow { expression } => {
                format!("the answer overflows a FLOAT: {expression}")
            }
            Refusal::Drained { operand } => {
                format!("{operand} must be a LIST or METRICS; a drained series is not allowed here")
            }
        }
    }

    /// The error `word` fails with: `<WORD>: E1nn <text>`.
    pub fn error(&self, word: &str) -> Error {
        Error(format!(
            "{}: {} {}",
            word.to_uppercase(),
            self.code(),
            self.text()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Refusal;

    /// Criterion 3: every code, rendered by its template.
    #[test]
    fn every_code_renders_its_template() {
        let s = |x: &str| x.to_string();
        let cases = [
            (
                Refusal::Domain {
                    operand: s("x"),
                    function: s("math.lgamma"),
                    value: s("0"),
                },
                "MATH.LGAMMA: E101 x is outside math.lgamma's domain: 0",
            ),
            (
                Refusal::Disagree {
                    what: s("lengths"),
                    n: 3,
                    m: 4,
                },
                "MATH.LGAMMA: E102 lengths do not agree: 3 and 4",
            ),
            (
                Refusal::NotNumbers {
                    operand: s("A"),
                    container: "MATRIX",
                    cell: s("\"x\""),
                },
                "MATH.LGAMMA: E103 A is not a MATRIX of numbers: \"x\"",
            ),
            (
                Refusal::Parameters {
                    family: s("normal"),
                    message: s("std_dev must be positive"),
                },
                "MATH.LGAMMA: E104 invalid normal parameters: std_dev must be positive",
            ),
            (
                Refusal::Undefined {
                    family: s("multinomial"),
                    operation: s("cdf"),
                },
                "MATH.LGAMMA: E105 multinomial has no cdf",
            ),
            (
                Refusal::Singular,
                "MATH.LGAMMA: E106 the matrix is singular",
            ),
            (
                Refusal::NotSymmetric {
                    i: 0,
                    j: 1,
                    by: 0.5,
                },
                "MATH.LGAMMA: E107 the matrix is not symmetric: [0,1] and [1,0] differ by 0.5",
            ),
            (
                Refusal::Complex {
                    re: -0.5,
                    im: -0.25,
                },
                "MATH.LGAMMA: E108 an eigenvalue is complex: -0.5 ± 0.25i",
            ),
            (
                Refusal::NotUnique {
                    first: 1.0,
                    second: -1.0,
                },
                "MATH.LGAMMA: E108 the dominant eigenvalue is not unique: 1, -1",
            ),
            (
                Refusal::Alphabet {
                    ch: 'Z',
                    pos: 4,
                    alphabet: s("dna"),
                },
                "MATH.LGAMMA: E109 Z at 4 is not in the dna alphabet",
            ),
            (
                Refusal::NoConvergence {
                    method: s("the Schur decomposition"),
                    cap: 3000,
                },
                "MATH.LGAMMA: E110 the Schur decomposition did not converge within 3000 iterations",
            ),
            (
                Refusal::Cap {
                    what: s("the number of steps"),
                    cap: 1_000_000,
                },
                "MATH.LGAMMA: E111 the number of steps exceeds its cap of 1000000",
            ),
            (
                Refusal::Model {
                    left: s("an INTEGER"),
                    expected: s("a LIST of 3 FLOATs"),
                },
                "MATH.LGAMMA: E112 the model lambda left an INTEGER, where a LIST of 3 FLOATs was required",
            ),
            (
                Refusal::Newick {
                    pos: 7,
                    why: s("an unclosed parenthesis"),
                },
                "MATH.LGAMMA: E113 malformed Newick at 7: an unclosed parenthesis",
            ),
            (
                Refusal::Overflow {
                    expression: s("Γ(172)"),
                },
                "MATH.LGAMMA: E114 the answer overflows a FLOAT: Γ(172)",
            ),
            (
                Refusal::Drained { operand: s("a") },
                "MATH.LGAMMA: E115 a must be a LIST or METRICS; a drained series is not allowed here",
            ),
        ];
        let mut codes: Vec<&str> = cases.iter().map(|(r, _)| r.code()).collect();
        codes.dedup();
        assert_eq!(codes.len(), 15, "E101 to E115, each at least once");
        for (r, want) in cases {
            assert_eq!(r.error("math.lgamma").0, want);
        }
    }
}
