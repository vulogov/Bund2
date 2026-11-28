// Bund, Word by Word : master file.
//
// Compile from the repository root, so the logo and the portrait under
// docs/assets resolve:
//
//   typst compile --root . books/Introduction/Introduction.typ
//
// or run books/Introduction/build.sh, which also refreshes the screens.
//
// A first book of the Bund language. Where the registers and RFCs are topical,
// this book follows ONE small job — a harbour's ledger — from the first string
// printed to a finished program, meeting each part of the language in the
// order a newcomer needs it. BIBLE.md fixes the example; read it before
// changing a chapter.
//
// Every terminal screen is captured from the real binary by tools/shoot.py
// (sessions/ -> screens/). None is typed by hand.

#import "design.typ": *

#book((
  include "chapters/preface-why-bund.typ",
  include "chapters/00-one-harbour-one-ledger.typ",

  part(number: "I", title: "First Words"),
  include "chapters/01-a-first-conversation.typ",
  include "chapters/02-the-stack.typ",
  include "chapters/03-the-top-speaks-first.typ",

  part(number: "II", title: "Teaching Bund New Words"),
  include "chapters/04-words-of-your-own.typ",
  include "chapters/05-decisions.typ",
  include "chapters/06-again-and-again.typ",

  part(number: "III", title: "Things to Reckon With"),
  include "chapters/07-text.typ",
  include "chapters/08-lists.typ",
  include "chapters/09-records.typ",

  part(number: "IV", title: "More Than One Stack"),
  include "chapters/10-the-counting-table.typ",
  include "chapters/11-the-other-quays.typ",

  part(number: "V", title: "The Ledger"),
  include "chapters/12-the-ledger.typ",
  include "chapters/13-under-the-hood.typ",

  part(number: "VI", title: "Where Bund Comes From"),
  include "chapters/14-the-stepping-stones.typ",

  part(number: "VII", title: "Reference"),
  include "chapters/A-the-words-of-this-book.typ",
  include "chapters/B-who-speaks-first.typ",
  include "chapters/C-answers.typ",

  include "chapters/99-about.typ",
  include "chapters/99b-about-the-author.typ",
))
