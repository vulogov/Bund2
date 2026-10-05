//! The Porter stemmer, for `string.tokenize.stemmed`.
//!
//! # Why this is here rather than a crate
//!
//! The reference stems with `rnltk::stem::get`
//! (`reference/Bund/src/stdlib/functions/string/tokenize.rs`, the
//! `SimpleStemmed` arm). `rnltk` declares `nalgebra`, `csv`, `serde_json`,
//! `regex`, `serde` and `thiserror` unconditionally -- it has no features --
//! so taking its 579-line string-only stemmer means compiling a linear-algebra
//! crate for one word. `rust-stemmers`, already in the tree under `natural`,
//! offers only Snowball `English`, which is Porter2 and **disagrees**:
//! `happily` stems to `happi` there and to `happili` here. The repository
//! owner's disposition was to write the algorithm out.
//!
//! # What verifies it, given that no golden can
//!
//! F74: the word's answer is built by iterating a `HashSet`, so its order
//! differs every run and no golden may capture it. That is a fact about the
//! *list*, not about stemming -- **one word in, one word out is perfectly
//! deterministic** -- so [`tests::every_stem_matches_the_oracle`] pins a
//! 98-word table measured from the oracle on 2026-10-05, one word per call so
//! no set is involved. That table is the contract this file has instead of a
//! golden, and it exists because the alternative was a divergence nothing
//! would ever catch.
//!
//! **A wider check was run once and is not kept.** 3,000 words from
//! `/usr/share/dict/words` were stemmed by the oracle and compared: **zero
//! disagreements**. Pinning all three thousand would put a 3,000-line table in
//! the crate for a word no program in the corpus calls, so the 98 stay as the
//! regression test and this paragraph records the breadth. To repeat it, take
//! a word list, generate one `"<word>" string.tokenize.stemmed println` per
//! line, and diff against [`porter`].
//!
//! # The algorithm
//!
//! Porter 1980, in the reference implementation's shape: `b` holds the word,
//! `k` is the index of its last character and `j` the end of the stem a test
//! is measuring. Both are `isize` because `j` reaches -1 for a suffix as long
//! as the word -- `ies` on a three-letter word -- and the C original relies on
//! that.
//!
//! Only ASCII letters are stemmed. A word holding anything else is returned
//! unchanged, which is also what keeps this byte-indexed rather than
//! char-indexed.

/// Stem one word. The input is lower-cased first, as `rnltk::stem::get` does.
pub(crate) fn porter(word: &str) -> String {
    let lower = word.to_lowercase();
    // The reference implementation stems nothing of length 2 or less, and this
    // is byte-indexed, so anything outside ASCII lower-case is left alone.
    if lower.len() <= 2 || !lower.bytes().all(|c| c.is_ascii_lowercase()) {
        return lower;
    }
    let mut s = Stemmer {
        b: lower.into_bytes(),
        k: 0,
        j: 0,
    };
    s.k = s.b.len() as isize - 1;
    s.step1ab();
    s.step1c();
    s.step2();
    s.step3();
    s.step4();
    s.step5();
    let end = (s.k + 1) as usize;
    String::from_utf8_lossy(&s.b[..end]).into_owned()
}

struct Stemmer {
    b: Vec<u8>,
    k: isize,
    j: isize,
}

impl Stemmer {
    fn at(&self, i: isize) -> u8 {
        self.b[i as usize]
    }

    /// Is the character at `i` a consonant? `y` is one unless what precedes it
    /// is, which is why this recurses exactly one step.
    fn cons(&self, i: isize) -> bool {
        match self.at(i) {
            b'a' | b'e' | b'i' | b'o' | b'u' => false,
            b'y' => {
                if i == 0 {
                    true
                } else {
                    !self.cons(i - 1)
                }
            }
            _ => true,
        }
    }

    /// Porter's *m*: the number of consonant-vowel sequences in `b[0..=j]`.
    fn m(&self) -> usize {
        let mut n = 0;
        let mut i = 0;
        loop {
            if i > self.j {
                return n;
            }
            if !self.cons(i) {
                break;
            }
            i += 1;
        }
        i += 1;
        loop {
            loop {
                if i > self.j {
                    return n;
                }
                if self.cons(i) {
                    break;
                }
                i += 1;
            }
            i += 1;
            n += 1;
            loop {
                if i > self.j {
                    return n;
                }
                if !self.cons(i) {
                    break;
                }
                i += 1;
            }
            i += 1;
        }
    }

    /// Does `b[0..=j]` hold a vowel?
    fn vowel_in_stem(&self) -> bool {
        (0..=self.j).any(|i| !self.cons(i))
    }

    /// Is `b[i]` the second of a doubled consonant?
    fn doublec(&self, i: isize) -> bool {
        i >= 1 && self.at(i) == self.at(i - 1) && self.cons(i)
    }

    /// Porter's *o*: consonant-vowel-consonant where the last is not w, x or y.
    fn cvc(&self, i: isize) -> bool {
        if i < 2 || !self.cons(i) || self.cons(i - 1) || !self.cons(i - 2) {
            return false;
        }
        !matches!(self.at(i), b'w' | b'x' | b'y')
    }

    /// Does the word end with `s`? Sets `j` to the end of the stem if so.
    fn ends(&mut self, s: &str) -> bool {
        let len = s.len() as isize;
        if len > self.k + 1 {
            return false;
        }
        let start = (self.k + 1 - len) as usize;
        if &self.b[start..=(self.k as usize)] == s.as_bytes() {
            self.j = self.k - len;
            true
        } else {
            false
        }
    }

    /// Replace the suffix after `j` with `s`, leaving `k` at the new end.
    fn setto(&mut self, s: &str) {
        let at = (self.j + 1) as usize;
        self.b.truncate(at);
        self.b.extend_from_slice(s.as_bytes());
        self.k = self.b.len() as isize - 1;
    }

    /// `setto`, but only when the stem's measure is positive.
    fn r(&mut self, s: &str) {
        if self.m() > 0 {
            self.setto(s);
        }
    }

    /// Plurals, and past participles.
    fn step1ab(&mut self) {
        if self.at(self.k) == b's' {
            if self.ends("sses") {
                self.k -= 2;
            } else if self.ends("ies") {
                self.setto("i");
            } else if self.at(self.k - 1) != b's' {
                self.k -= 1;
            }
        }
        if self.ends("eed") {
            if self.m() > 0 {
                self.k -= 1;
            }
        } else if (self.ends("ed") || self.ends("ing")) && self.vowel_in_stem() {
            self.k = self.j;
            if self.ends("at") {
                self.setto("ate");
            } else if self.ends("bl") {
                self.setto("ble");
            } else if self.ends("iz") {
                self.setto("ize");
            } else if self.doublec(self.k) {
                self.k -= 1;
                if matches!(self.at(self.k), b'l' | b's' | b'z') {
                    self.k += 1;
                }
            } else if self.m() == 1 && self.cvc(self.k) {
                self.setto("e");
            }
        }
    }

    /// A terminal `y` becomes `i` when the stem holds a vowel.
    fn step1c(&mut self) {
        if self.ends("y") && self.vowel_in_stem() {
            self.b[self.k as usize] = b'i';
        }
    }

    /// Double suffices to single ones.
    ///
    /// **Flat rather than Porter's `switch` on the penultimate letter.** That
    /// switch is the C original's jump table and nothing else: every `ends`
    /// test below is exact, so the dispatch cannot change which one matches.
    /// Two orderings *are* load-bearing and are kept -- `ational` before
    /// `tional`, and `ization` before `ation`, because each of the first pair
    /// ends with the second.
    fn step2(&mut self) {
        if self.k == 0 {
            return;
        }
        if self.ends("ational") {
            self.r("ate");
        } else if self.ends("tional") {
            self.r("tion");
        } else if self.ends("enci") {
            self.r("ence");
        } else if self.ends("anci") {
            self.r("ance");
        } else if self.ends("izer") {
            self.r("ize");
        } else if self.ends("bli") {
            self.r("ble");
        } else if self.ends("alli") {
            self.r("al");
        } else if self.ends("entli") {
            self.r("ent");
        } else if self.ends("eli") {
            self.r("e");
        } else if self.ends("ousli") {
            self.r("ous");
        } else if self.ends("ization") {
            self.r("ize");
        } else if self.ends("ation") || self.ends("ator") {
            // Both map to `ate`; `ends` sets `j` for whichever matched, and
            // `||` short-circuits, so the second is only tried when the first
            // missed.
            self.r("ate");
        } else if self.ends("alism") {
            self.r("al");
        } else if self.ends("iveness") {
            self.r("ive");
        } else if self.ends("fulness") {
            self.r("ful");
        } else if self.ends("ousness") {
            self.r("ous");
        } else if self.ends("aliti") {
            self.r("al");
        } else if self.ends("iviti") {
            self.r("ive");
        } else if self.ends("biliti") {
            self.r("ble");
        } else if self.ends("logi") {
            self.r("log");
        }
    }

    /// `-ic-`, `-full`, `-ness`. Flat for the same reason as [`Self::step2`];
    /// these seven suffices are pairwise non-overlapping, so no ordering here
    /// is load-bearing.
    fn step3(&mut self) {
        // The three that map to `ic` and the three that map to nothing are
        // merged, which is safe here and only here: step 3's seven suffices
        // are pairwise non-overlapping, so no one of them can shadow another
        // and the order among them cannot matter.
        if self.ends("icate") || self.ends("iciti") || self.ends("ical") {
            self.r("ic");
        } else if self.ends("ative") || self.ends("ful") || self.ends("ness") {
            self.r("");
        } else if self.ends("alize") {
            self.r("al");
        }
    }

    /// `<c>vcvc<v>` suffices.
    fn step4(&mut self) {
        if self.k == 0 {
            return;
        }
        let matched = match self.at(self.k - 1) {
            b'a' => self.ends("al"),
            b'c' => self.ends("ance") || self.ends("ence"),
            b'e' => self.ends("er"),
            b'i' => self.ends("ic"),
            b'l' => self.ends("able") || self.ends("ible"),
            b'n' => {
                self.ends("ant")
                    || self.ends("ement")
                    || self.ends("ment")
                    || self.ends("ent")
            }
            b'o' => {
                if self.ends("ion") && self.j >= 0 && matches!(self.at(self.j), b's' | b't') {
                    true
                } else {
                    self.ends("ou")
                }
            }
            b's' => self.ends("ism"),
            b't' => self.ends("ate") || self.ends("iti"),
            b'u' => self.ends("ous"),
            b'v' => self.ends("ive"),
            b'z' => self.ends("ize"),
            _ => false,
        };
        if matched && self.m() > 1 {
            self.k = self.j;
        }
    }

    /// A terminal `e`, and a doubled `l`.
    fn step5(&mut self) {
        self.j = self.k;
        if self.at(self.k) == b'e' {
            let a = self.m();
            if a > 1 || (a == 1 && !self.cvc(self.k - 1)) {
                self.k -= 1;
            }
        }
        if self.at(self.k) == b'l' && self.doublec(self.k) && self.m() > 1 {
            self.k -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::porter;

    /// **The table this file has instead of a golden.** 98 words with the
    /// stem the oracle answered on 2026-10-05, captured one word per call --
    /// `"<word>" string.tokenize.stemmed` -- so no `HashSet` is involved and
    /// F74's ordering does not arise. The vocabulary is Porter's own worked
    /// examples plus the cases that separate Porter from Porter2.
    ///
    /// If this fails, the stemmer has drifted from the reference and **no
    /// golden would have told you**: the word's list order is unreproducible,
    /// so conformance cannot see it. That is the whole reason the table is
    /// here.
    #[test]
    fn every_stem_matches_the_oracle() {
        const TABLE: [(&str, &str); 98] = [
        ("a", "a"),
        ("activate", "activ"),
        ("adjustable", "adjust"),
        ("adjustment", "adjust"),
        ("adoption", "adopt"),
        ("agreed", "agre"),
        ("airliner", "airlin"),
        ("allowance", "allow"),
        ("am", "am"),
        ("analogousli", "analog"),
        ("and", "and"),
        ("angulariti", "angular"),
        ("are", "ar"),
        ("be", "be"),
        ("been", "been"),
        ("being", "be"),
        ("bled", "bled"),
        ("bowdlerize", "bowdler"),
        ("callousness", "callous"),
        ("caress", "caress"),
        ("caresses", "caress"),
        ("cats", "cat"),
        ("cease", "ceas"),
        ("communism", "commun"),
        ("conditional", "condit"),
        ("conflated", "conflat"),
        ("conformabli", "conform"),
        ("controll", "control"),
        ("decisiveness", "decis"),
        ("defensible", "defens"),
        ("dependent", "depend"),
        ("differentli", "differ"),
        ("digitizer", "digit"),
        ("dogs", "dog"),
        ("effective", "effect"),
        ("electrical", "electr"),
        ("electriciti", "electr"),
        ("failing", "fail"),
        ("falling", "fall"),
        ("feed", "feed"),
        ("feudalism", "feudal"),
        ("filing", "file"),
        ("fizzed", "fizz"),
        ("flies", "fli"),
        ("formaliti", "formal"),
        ("formalize", "formal"),
        ("formative", "form"),
        ("goodness", "good"),
        ("gyroscopic", "gyroscop"),
        ("happily", "happili"),
        ("happy", "happi"),
        ("hesitanci", "hesit"),
        ("hissing", "hiss"),
        ("homologou", "homolog"),
        ("homologous", "homolog"),
        ("hopeful", "hope"),
        ("hopefulness", "hope"),
        ("hopping", "hop"),
        ("inference", "infer"),
        ("irritant", "irrit"),
        ("is", "is"),
        ("it", "it"),
        ("its", "it"),
        ("motoring", "motor"),
        ("not", "not"),
        ("of", "of"),
        ("operator", "oper"),
        ("or", "or"),
        ("plastered", "plaster"),
        ("ponies", "poni"),
        ("predication", "predic"),
        ("probate", "probat"),
        ("radicalli", "radic"),
        ("rate", "rate"),
        ("rational", "ration"),
        ("relational", "relat"),
        ("replacement", "replac"),
        ("revival", "reviv"),
        ("roll", "roll"),
        ("runner", "runner"),
        ("running", "run"),
        ("runs", "run"),
        ("sensibiliti", "sensibl"),
        ("sensitiviti", "sensit"),
        ("sing", "sing"),
        ("sized", "size"),
        ("sky", "sky"),
        ("studies", "studi"),
        ("tanned", "tan"),
        ("the", "the"),
        ("ties", "ti"),
        ("triplicate", "triplic"),
        ("troubled", "troubl"),
        ("valenci", "valenc"),
        ("vietnamization", "vietnam"),
        ("vileli", "vile"),
        ("was", "wa"),
        ("were", "were"),
        ];
        let mut wrong = Vec::new();
        for (word, want) in TABLE {
            let got = porter(word);
            if got != want {
                wrong.push(format!("{word}: got {got}, oracle says {want}"));
            }
        }
        assert!(wrong.is_empty(), "{} of 98 disagree:\n{}", wrong.len(), wrong.join("\n"));
    }

    /// `rnltk::stem::get` lower-cases before stemming, and words of two
    /// characters or fewer are returned untouched.
    #[test]
    fn case_is_folded_and_short_words_are_left_alone() {
        assert_eq!(porter("RUNNING"), "run");
        assert_eq!(porter("Flies"), "fli");
        for w in ["a", "an", "is", "be", ""] {
            assert_eq!(porter(w), w, "{w} is too short to stem");
        }
        // Not ASCII lower-case after folding, so returned as it is.
        assert_eq!(porter("na\u{ef}ve"), "na\u{ef}ve");
    }
}
