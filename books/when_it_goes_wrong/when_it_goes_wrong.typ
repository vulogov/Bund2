// Bund, When It Goes Wrong : master file.
//
// Compile from the repository root, so the logo and the portrait under
// docs/assets resolve:
//
//   typst compile --root . --font-path books/Introduction/assets \
//       books/when_it_goes_wrong/when_it_goes_wrong.typ
//
// or run books/when_it_goes_wrong/build.sh, which also refreshes the screens.
//
// The fourth book of the harbour. Three kinds of trouble: a program that
// stops, a program that is wrong, a program that is slow. And every means
// bund2 0.57.0 has of finding out why: the error report, `?try` and `raise`,
// the log words, `check` and `infer`, the looking words, the trace, the
// debugger with its breakpoints and watches, and the profiler. BIBLE.md
// fixes the programs and what each is for; read it before changing one.
//
// Every terminal screen is captured from the real binary by tools/shoot.py
// (sessions/ -> screens/). None is typed by hand.

#import "design.typ": *

#book((
  include "chapters/preface.typ",
  include "chapters/00-three-kinds-of-wrong.typ",

  part(number: "I", title: "It Stops"),
  include "chapters/01-reading-the-report.typ",
  include "chapters/02-the-net-and-the-bell.typ",
  include "chapters/03-before-it-runs.typ",

  part(number: "II", title: "Looking"),
  include "chapters/04-looking.typ",
  include "chapters/05-the-trace.typ",

  part(number: "III", title: "Stopping It Yourself"),
  include "chapters/06-the-debugger.typ",
  include "chapters/07-breakpoints.typ",
  include "chapters/08-watches.typ",
  include "chapters/09-from-inside.typ",

  part(number: "IV", title: "It Is Slow"),
  include "chapters/10-the-profile.typ",

  part(number: "V", title: "Choosing"),
  include "chapters/11-which-tool.typ",

  part(number: "VI", title: "Reference"),
  include "chapters/A-everything-on-one-page.typ",
  include "chapters/B-the-programs.typ",

  include "chapters/99-about.typ",
  include "../Introduction/chapters/99b-about-the-author.typ",
))
