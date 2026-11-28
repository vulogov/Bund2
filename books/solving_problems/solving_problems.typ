// Bund, Problem by Problem : master file.
//
// Compile from the repository root, so the logo and the portrait under
// docs/assets resolve:
//
//   typst compile --root . --font-path books/Introduction/assets \
//       books/solving_problems/solving_problems.typ
//
// or run books/solving_problems/build.sh, which also refreshes the screens.
//
// The companion to "Bund, Word by Word": the same harbour and no new story,
// only problems. Each chapter file under chapters/ defines a title, an
// introduction and an array of problems; this file lays that array out three
// times, as problems, as hints and as answers. BIBLE.md says what a problem
// must be; read it before adding one.
//
// Every terminal screen is captured from the real binary by tools/shoot.py
// (sessions/ -> screens/). None is typed by hand.

#import "design.typ": *

#import "chapters/01-crates.typ" as c01
#import "chapters/02-pennies.typ" as c02
#import "chapters/03-words.typ" as c03
#import "chapters/04-questions.typ" as c04
#import "chapters/05-rounds.typ" as c05
#import "chapters/06-text.typ" as c06
#import "chapters/07-lists.typ" as c07
#import "chapters/08-records.typ" as c08
#import "chapters/09-table-and-quays.typ" as c09
#import "chapters/10-a-word-that-calls-itself.typ" as c10
#import "chapters/11-programs-as-values.typ" as c11
#import "chapters/12-harbour-days.typ" as c12

#let chapters = (c01, c02, c03, c04, c05, c06, c07, c08, c09, c10, c11, c12)

// The number of each chapter's first problem.
#let firsts = {
  let out = ()
  let n = 1
  for ch in chapters {
    out.push(n)
    n += ch.problems.len()
  }
  out
}

#let laid(render, from, to) = range(from, to).map(i =>
  render(i + 1, chapters.at(i), firsts.at(i)))

#book((
  include "chapters/00-before-you-begin.typ",

  part(number: "I", title: "The Problems"),
  ..laid(problem_chapter, 0, 12),

  part(number: "II", title: "The Hints"),
  include "chapters/H-hints.typ",
  ..laid(hint_chapter, 0, 12),

  part(number: "III", title: "The Answers"),
  include "chapters/S-answers.typ",
  ..laid(answer_chapter, 0, 12),

  part(number: "IV", title: "Reference"),
  include "chapters/A-harbour-words.typ",

  include "chapters/99-about.typ",
  include "../Introduction/chapters/99b-about-the-author.typ",
))
