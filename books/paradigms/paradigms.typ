// Bund, Four Ways : master file.
//
// Compile from the repository root, so the logo and the portrait under
// docs/assets resolve:
//
//   typst compile --root . --font-path books/Introduction/assets \
//       books/paradigms/paradigms.typ
//
// or run books/paradigms/build.sh, which also refreshes the screens.
//
// The third book of the harbour. One job, the day's ledger under a tariff
// by kind of cargo, done four times: by commands, by functions, by objects
// and by rules. Each version prints the same page and is put to the same
// two tests. BIBLE.md fixes the job and the numbers; read it before
// changing a program.
//
// Every terminal screen is captured from the real binary by tools/shoot.py
// (sessions/ -> screens/). None is typed by hand.

#import "design.typ": *

#book((
  include "chapters/preface.typ",
  include "chapters/00-the-job.typ",

  part(number: "I", title: "Steps and Values"),
  include "chapters/01-by-commands.typ",
  include "chapters/02-by-functions.typ",

  part(number: "II", title: "Things and Facts"),
  include "chapters/03-by-objects.typ",
  include "chapters/04-by-rules.typ",

  part(number: "III", title: "Choosing"),
  include "chapters/05-side-by-side.typ",
  include "chapters/06-all-four-at-once.typ",

  include "chapters/99-about.typ",
  include "../Introduction/chapters/99b-about-the-author.typ",
))
