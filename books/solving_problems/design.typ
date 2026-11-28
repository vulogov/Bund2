// Bund, Problem by Problem : design tokens + page chrome.
//
// The design of "Bund, Problem by Problem", with the three things a problem book
// adds: a problem, a hint and an answer. `session()` is a screen whose text
// was captured from the real binary by tools/shoot.py.
//
// Compile from the repository root, so the shared assets under docs/ resolve:
//   typst compile --root . books/solving_problems/solving_problems.typ

#let book_title    = "Bund, Problem by Problem"
#let book_subtitle = "Seventy-Three Problems for the Harbourmaster, Every Answer Run"
#let book_eyebrow  = "A Second Book of the Harbour"
#let book_author   = "Vladimir Ulogov"
#let book_year     = "2026"
#let book_version  = "bund2 0.57.0"

// The project mark and the author's portrait live with the project's other
// assets. Paths are relative to this file.
#let bund_logo       = "../../docs/assets/logo/bund2-logo-transparent.png"
#let author_portrait = "../../docs/assets/author-portrait.png"

// ── Palette — warm paper, cool ink, one harbour-water accent ─────────
#let ink_black   = rgb("#1a1a1a")
#let ink_gray    = rgb("#5d5d5d")
#let ink_faint   = rgb("#9a9a9a")
#let ink_rule    = rgb("#c6c0b5")
#let ink_accent  = rgb("#1f4e5f")            // harbour teal — chapter numbers
#let ink_smoke   = rgb("#5f6b6e")            // slate — cover eyebrow + screen bar
#let ink_paper   = rgb("#fdfaf3")            // warm cream — cover ground
#let ink_term    = rgb("#1f4e5f")            // teal — term definitions
#let ink_code_bg = rgb("#f3eee4")
#let ink_call_bg = rgb("#f6f1e6")
#let ink_term_bg = rgb("#e8f0f1")
#let ink_recap   = rgb("#3f6b4a")            // muted green — recap accent
#let ink_recap_bg = rgb("#e9f3ea")
#let ink_warn    = rgb("#8a6d1f")            // amber — "who speaks first" traps
#let ink_warn_bg = rgb("#f6f0df")
#let ink_prob    = rgb("#6b4a6e")            // plum — problems
#let ink_prob_bg = rgb("#f2ecf3")

// The text face is Kyiv Machine, a typewriter face with one weight and no
// italic. The build reads it from the first book's assets (--font-path). It draws
// braces and brackets as parentheses and the asterisk as a bullet, which a
// book about Bund cannot have, so those characters are withheld from it and
// fall through to the next family. Code is never set in it.
#let body_family = (
  (name: "Kyiv Machine", covers: regex("[^{}\\[\\]*~^]")),
  "Libertinus Serif",
)
#let mono_family = ("DejaVu Sans Mono",)

#let book_page = (
  paper: "iso-b5",
  margin: (inside: 26mm, outside: 20mm, top: 22mm, bottom: 24mm),
)

// ── Part divider ────────────────────────────────────────────────────
#let part(number: "I", title: "") = {
  pagebreak(weak: true)
  hide(heading(level: 1, numbering: none, outlined: true, bookmarked: true, [Part #number — #title]))
  v(6cm)
  align(center)[
    #image(bund_logo, width: 18mm)
    #v(7mm)
    #text(font: body_family, size: 11pt, tracking: 3pt, fill: ink_gray, upper("Part " + number))
    #v(6mm)
    #line(length: 36%, stroke: 0.5pt + ink_rule)
    #v(6mm)
    #text(font: body_family, size: 26pt, weight: "bold", fill: ink_black, title)
  ]
}

// ── Chapter / appendix opening ──────────────────────────────────────
#let opening(kind, mark, title) = {
  pagebreak(weak: true)
  hide(heading(level: 1, numbering: none, outlined: true, bookmarked: true,
    if kind == "Chapter" [#mark — #title] else [Appendix #mark — #title]))
  v(1.2cm)
  {
    set par(first-line-indent: 0pt, justify: false)
    stack(dir: ttb, spacing: 5mm,
      text(font: body_family, size: 9pt, tracking: 2pt, fill: ink_gray, upper(kind + " " + mark)),
      text(font: body_family, size: 84pt, weight: "bold", fill: ink_accent, mark),
      text(font: body_family, size: 25pt, weight: "regular", fill: ink_black, title),
    )
  }
  v(8mm)
  line(length: 100%, stroke: 0.5pt + ink_rule)
  v(8mm)
}
#let chapter(number: 0, title: "") = opening("Chapter", str(number), title)
#let appendix(letter: "A", title: "") = opening("Appendix", letter, title)

// ── Section / subsection ────────────────────────────────────────────
#let section(title) = {
  hide(heading(level: 2, numbering: none, outlined: true, title))
  block(
    sticky: true, above: 8mm, below: 3.2mm,
    text(font: body_family, size: 15pt, weight: "bold", fill: ink_black, title),
  )
}
#let subsection(title) = {
  block(
    sticky: true, above: 5.5mm, below: 2.4mm,
    text(font: body_family, size: 11.5pt, weight: "bold", fill: ink_black, title),
  )
}

// ── Boxes: one shape, four colours ──────────────────────────────────
#let _box(label, fg, bg, body, head: none, breakable: false) = {
  v(2mm)
  block(
    fill: bg, stroke: (left: 2pt + fg),
    inset: (left: 9pt, right: 9pt, top: 7pt, bottom: 7pt),
    width: 100%, radius: 1pt, breakable: breakable,
    {
      set par(first-line-indent: 0pt)
      text(font: body_family, size: 8pt, weight: "bold", fill: fg, tracking: 1.5pt, upper(label))
      if head != none {
        h(6pt)
        text(font: body_family, size: 11pt, weight: "bold", fill: fg, head)
      }
      v(2mm)
      body
    },
  )
  v(2mm)
}

// DEFINE a term, once, where it first matters.
#let term(name, body) = _box("Term", ink_term, ink_term_bg, body, head: name)
// An aside. Label it by reader: "For Forth hands", "For the curious".
#let callout(label: "Note", body) = _box(label, ink_accent, ink_call_bg, body)
// A trap. Nearly always: who speaks first.
#let warn(label: "Watch out", body) = _box(label, ink_warn, ink_warn_bg, body)

// ── Chapter-end recap ───────────────────────────────────────────────
#let recap(items) = {
  v(5mm)
  _box("What you learned", ink_recap, ink_recap_bg, list(..items))
}

// ── Terminal screen — a faithful monospace rendering of a terminal.
//    `body` is a raw block; `caption` names it. ────────────────────────
#let _in_screen = state("in-screen", false)
#let screen(caption: "", breakable: false, body) = {
  v(2mm)
  block(breakable: breakable, width: 100%, {
    set block(spacing: 0pt)
    block(
      fill: ink_smoke,
      inset: (left: 8pt, right: 8pt, top: 3pt, bottom: 3pt),
      width: 100%,
      radius: (top-left: 2pt, top-right: 2pt),
      {
        text(font: mono_family, size: 8pt, fill: ink_paper, "● ● ●")
        h(6pt)
        text(font: body_family, size: 8.5pt, style: "italic", fill: ink_paper, caption)
      },
    )
    block(
      fill: ink_code_bg,
      stroke: 0.5pt + ink_rule,
      inset: 8pt,
      width: 100%,
      breakable: breakable,
      radius: (bottom-left: 2pt, bottom-right: 2pt),
      { _in_screen.update(true); text(font: mono_family, size: 8pt, body); _in_screen.update(false) },
    )
  })
  v(2mm)
}

// A screen whose text was captured from the real binary: screens/<name>.txt,
// written by tools/shoot.py from sessions/<name>. Never typed by hand.
#let session(name, caption: "") = screen(
  caption: caption,
  raw(read("screens/" + name + ".txt").trim("\n", at: end), block: true),
)

// A source file from code/, printed whole.
#let listing(name, caption: "") = screen(
  breakable: true,
  caption: if caption == "" { name } else { caption },
  raw(read("code/" + name).trim("\n", at: end), block: true),
)

// ── Word table — the words a section introduced, with stack effects ──
#let words(rows) = {
  v(2mm)
  block(width: 100%, breakable: false, stroke: (top: 0.5pt + ink_rule, bottom: 0.5pt + ink_rule),
    inset: (y: 5pt), {
    for (name, effect, desc) in rows {
      grid(columns: (33mm, 1fr), gutter: 3mm,
        text(font: mono_family, weight: "bold", size: 8.5pt, fill: ink_accent, name),
        {
          set par(first-line-indent: 0pt, justify: false)
          if effect != "" {
            text(font: mono_family, size: 8pt, fill: ink_gray, effect)
            h(2.5mm)
          }
          text(font: body_family, size: 10pt, fill: ink_black, desc)
        })
      v(1.4mm)
    }
  })
  v(2mm)
}

// Two-column reference rows (appendices).
#let chord_row(name, desc) = (name, desc)
#let chord_table(rows) = block(width: 100%, {
  for (name, desc) in rows {
    block(breakable: false, width: 100%, {
      grid(columns: (38mm, 1fr), gutter: 4mm,
        text(font: mono_family, weight: "bold", size: 9pt, fill: ink_black, name),
        text(font: body_family, size: 10pt, fill: ink_black, desc))
      v(1.6mm)
    })
  }
})

// ── Stack pictures — the stack drawn as crates, before and after.
//    piles(("3 4", ("3", "4")), ("+", ("7",)))  : one pile per step, each a
//    label and its values from the bottom up. The top crate is inked. ──
#let _crate(v, top, w) = box(
  width: w, height: 6mm,
  fill: if top { ink_term_bg } else { white },
  stroke: (if top { 0.9pt } else { 0.5pt }) + (if top { ink_accent } else { ink_gray }),
  align(center + horizon, text(font: mono_family, size: 8pt, fill: ink_black, v)),
)
#let piles(..steps, note: none, width: 17mm) = {
  v(3mm)
  block(breakable: false, width: 100%, align(center, {
    let cols = steps.pos().map(((label, vals)) => {
      let n = vals.len()
      stack(dir: ttb, spacing: 1.5mm,
        if n == 0 {
          box(width: width, height: 6mm, align(center + horizon,
            text(font: body_family, size: 8pt, style: "italic", fill: ink_faint, "empty")))
        } else {
          stack(dir: ttb, spacing: 0.7mm,
            ..range(n).rev().map(i => _crate(vals.at(i), i == n - 1, width)))
        },
        line(length: width + 4mm, stroke: 1pt + ink_black),
        text(font: mono_family, size: 8pt, weight: "bold", fill: ink_accent, label),
      )
    })
    grid(columns: cols.len(), column-gutter: 7mm, align: bottom, ..cols)
    if note != none {
      v(2mm)
      text(font: body_family, style: "italic", size: 9pt, fill: ink_gray, note)
    }
  }))
  v(3mm)
}

// ── Small helpers ───────────────────────────────────────────────────
#let dropcap(letter) = box(baseline: 0.62em,
  text(font: body_family, size: 2.7em, weight: "bold", fill: ink_accent, letter))

#let figure_note(body) = align(center,
  text(font: body_family, style: "italic", size: 9pt, fill: ink_gray, body))

#let brand_mark(width: 22mm) = align(center, image(bund_logo, width: width))
#let portrait(width: 100%) = image(author_portrait, width: width)

// An unnumbered front- or back-matter title.
#let plain_title(eyebrow, title) = {
  pagebreak(weak: true)
  hide(heading(level: 1, numbering: none, outlined: true, bookmarked: true, title))
  v(1.4cm)
  {
    set par(first-line-indent: 0pt, justify: false)
    stack(dir: ttb, spacing: 4mm,
      text(font: body_family, size: 9pt, tracking: 2pt, fill: ink_gray, upper(eyebrow)),
      text(font: body_family, size: 25pt, weight: "bold", fill: ink_accent, title))
  }
  v(6mm)
  line(length: 100%, stroke: 0.5pt + ink_rule)
  v(8mm)
}

// ── The problem book's own pieces ───────────────────────────────────
//
// A chapter of problems is a file that defines three things:
//
//   title     the chapter's title
//   intro     what the chapter is about, and the words it leans on
//   problems  an array of dictionaries, one per problem:
//               id      names sessions/<id> and screens/<id>.txt
//               title   a few words
//               grade   1, 2 or 3 crates
//               ask     the problem
//               hint    one nudge, or none
//               answer  what to notice in the answer's screen
//               file    for a whole program: its name in code/answers/
//
// The master file lays the same array out three times: as problems, as
// hints and as answers. A problem's number is its place in the book.

// How hard: one, two or three crates.
#let grade(n) = box(baseline: -0.4pt, stack(dir: ltr, spacing: 1.1mm,
  ..range(3).map(i => rect(width: 3.6mm, height: 2.5mm,
    stroke: 0.6pt + ink_prob, fill: if i < n { ink_prob } else { none }))))

#let problem(number, p) = {
  v(2mm)
  block(
    fill: ink_prob_bg, stroke: (left: 2pt + ink_prob),
    inset: (left: 9pt, right: 9pt, top: 7pt, bottom: 8pt),
    width: 100%, radius: 1pt, breakable: false,
    {
      set par(first-line-indent: 0pt)
      grid(columns: (auto, 1fr, auto), column-gutter: 3mm, align: horizon,
        text(font: body_family, size: 15pt, weight: "bold", fill: ink_prob, str(number)),
        text(font: body_family, size: 11pt, weight: "bold", fill: ink_black, p.title),
        grade(p.grade))
      v(2.2mm)
      p.ask
    },
  )
  v(2mm)
}

#let hint(number, p) = block(breakable: false, width: 100%, below: 3.2mm, {
  set par(first-line-indent: 0pt)
  grid(columns: (9mm, 1fr), column-gutter: 2mm,
    text(font: body_family, size: 11pt, weight: "bold", fill: ink_prob, str(number)),
    p.hint)
})

#let answer(number, p) = {
  block(sticky: true, above: 8mm, below: 2.6mm, {
    set par(first-line-indent: 0pt)
    text(font: body_family, size: 15pt, weight: "bold", fill: ink_prob, str(number))
    h(3mm)
    text(font: body_family, size: 11.5pt, weight: "bold", fill: ink_black, p.title)
  })
  if p.at("file", default: none) != none {
    listing("answers/" + p.file, caption: p.file)
  }
  session(p.id, caption: "Problem " + str(number))
  {
    set par(first-line-indent: 0pt)
    p.answer
  }
}

// A chapter of problems, its hints, its answers. `first` is the number of
// the chapter's first problem.
#let problem_chapter(number, ch, first) = {
  chapter(number: number, title: ch.title)
  ch.intro
  for (i, p) in ch.problems.enumerate() { problem(first + i, p) }
}
#let hint_chapter(number, ch, first) = {
  section("Chapter " + str(number) + ": " + ch.title)
  for (i, p) in ch.problems.enumerate() {
    if p.at("hint", default: none) != none { hint(first + i, p) }
  }
}
#let answer_chapter(number, ch, first) = {
  section("Chapter " + str(number) + ": " + ch.title)
  for (i, p) in ch.problems.enumerate() { answer(first + i, p) }
}

// ── Master document wrapper ─────────────────────────────────────────
#let book(pages) = {
  set document(title: book_title, author: book_author)
  set text(font: body_family, size: 10pt, fill: ink_black, lang: "en")
  set par(leading: 0.78em, justify: true, first-line-indent: 1em)
  // One weight, no italic: emphasis the way a typewriter makes it. Underline
  // for emphasis, a second strike for strong.
  show emph: it => underline(offset: 2pt, stroke: 0.5pt, it.body)
  show strong: it => text(stroke: 0.35pt + ink_black, it.body)
  // A raw block is boxed, except inside a screen, which is its own box.
  show raw.where(block: true): it => context {
    if _in_screen.get() {
      text(font: mono_family, size: 8pt, it)
    } else {
      block(
        fill: ink_code_bg, stroke: 0.5pt + ink_rule, inset: 7pt, radius: 2pt, width: 100%,
        breakable: false,
        text(font: mono_family, size: 8pt, it),
      )
    }
  }
  show raw.where(block: false): it => box(
    fill: ink_code_bg, inset: (x: 2pt, y: 0pt), outset: (y: 2pt), radius: 1pt,
    text(font: mono_family, size: 9.5pt, it),
  )

  // ── Cover ──
  set page(paper: book_page.paper, margin: 0pt, numbering: none, header: none, fill: ink_paper)
  block(width: 100%, height: 100%)[
    #place(top + left, dx: 12mm, dy: 12mm,
      rect(width: 100% - 24mm, height: 100% - 24mm, stroke: 1pt + ink_accent))
    #place(top + left, dx: 14mm, dy: 14mm,
      rect(width: 100% - 28mm, height: 100% - 28mm, stroke: 0.4pt + ink_accent))
    // Five crates, one lifted: the stack, on the cover.
    #place(top + center, dy: 30mm, {
      let crate(dx, dy) = place(top + center, dx: dx, dy: dy,
        rect(width: 9mm, height: 3.2mm, stroke: 0.7pt + ink_accent, fill: ink_paper))
      crate(0mm, 0mm); crate(0mm, 6mm); crate(0mm, 9.9mm); crate(0mm, 13.8mm); crate(0mm, 17.7mm)
      place(top + center, dy: 21.6mm, line(length: 22mm, stroke: 0.9pt + ink_accent))
    })
    #place(top + center, dy: 66mm, block(width: 74%)[
      #set par(justify: false)
      #align(center)[
        #text(font: body_family, size: 11pt, tracking: 4pt, fill: ink_smoke, upper(book_eyebrow))
        #v(11mm)
        #text(font: body_family, size: 32pt, weight: "bold", fill: ink_black, book_title)
        #v(6mm)
        #line(length: 55%, stroke: 0.6pt + ink_accent)
        #v(6mm)
        #text(font: body_family, size: 12.5pt, style: "italic", fill: ink_smoke, book_subtitle)
      ]
    ])
    #place(bottom + center, dy: -48mm, image(bund_logo, width: 24mm))
    #place(bottom + center, dy: -30mm, align(center)[
      #text(font: body_family, size: 10pt, fill: ink_smoke, book_author)
      #v(2mm)
      #text(font: body_family, size: 9pt, fill: ink_smoke, book_year + " · examples run on " + book_version)
    ])
  ]
  pagebreak()

  // Contents
  set page(margin: book_page.margin, fill: white)
  text(font: body_family, size: 22pt, weight: "bold", fill: ink_black, "Contents")
  v(7mm)
  outline(title: none, indent: auto, depth: 1)
  pagebreak()

  // Body
  set page(
    numbering: "1", number-align: center,
    header: context {
      if counter(page).get().first() > 1 {
        align(center, text(font: body_family, size: 8pt, fill: ink_faint, tracking: 1.5pt, upper(book_title)))
      }
    },
  )
  counter(page).update(1)
  for p in pages [ #p ]
}
