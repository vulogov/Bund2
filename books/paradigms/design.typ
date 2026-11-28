// Bund, Four Ways : design tokens + page chrome.
//
// The design of "Bund, Word by Word". `session()` is a screen whose text was
// captured from the real binary by tools/shoot.py, and `listing()` prints a
// file from code/.
//
// Compile from the repository root, so the shared assets under docs/ resolve:
//   typst compile --root . books/paradigms/paradigms.typ

#let book_title    = "Bund, Four Ways"
#let book_subtitle = "One Harbour Day by Commands, by Functions, by Objects and by Rules"
#let book_eyebrow  = "A Third Book of the Harbour"
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

// A source file from code/, printed whole. A file that fits on a page is
// never split across two. A file longer than a page has to be, and is split
// only at a blank line, so that no definition and no comment is cut in half.
#let _listing_max = 195mm
#let _long_listing(cap, src) = context {
  let mono(body) = text(font: mono_family, size: 8pt, body)
  // The blank line that divided two pieces is put back as space under the
  // first: two line pitches less the height of one line.
  let pitch = measure(mono(raw("a\n\nb", block: true))).height - measure(mono(raw("a\nb", block: true))).height
  let gap = 2 * pitch - measure(mono("a")).height
  let chunks = src.split(regex("\n[ \t]*\n"))
  let last = chunks.len() - 1
  let edge = 0.5pt + ink_rule
  v(2mm)
  {
    set block(spacing: 0pt)
    for (i, chunk) in chunks.enumerate() {
      block(breakable: false, width: 100%, {
        set block(spacing: 0pt)
        if i == 0 {
          block(
            fill: ink_smoke,
            inset: (left: 8pt, right: 8pt, top: 3pt, bottom: 3pt),
            width: 100%,
            radius: (top-left: 2pt, top-right: 2pt),
            {
              text(font: mono_family, size: 8pt, fill: ink_paper, "● ● ●")
              h(6pt)
              text(font: body_family, size: 8.5pt, style: "italic", fill: ink_paper, cap)
            },
          )
        }
        block(
          fill: ink_code_bg,
          stroke: (left: edge, right: edge,
            top: if i == 0 { edge } else { none },
            bottom: if i == last { edge } else { none }),
          inset: (left: 8pt, right: 8pt,
            top: if i == 0 { 8pt } else { 0pt },
            bottom: if i == last { 8pt } else { gap }),
          width: 100%,
          radius: if i == last { (bottom-left: 2pt, bottom-right: 2pt) } else { 0pt },
          { _in_screen.update(true); mono(raw(chunk, block: true)); _in_screen.update(false) },
        )
      })
    }
  }
  v(2mm)
}
#let listing(name, caption: "") = {
  let src = read("code/" + name).trim("\n", at: end)
  let cap = if caption == "" { name } else { caption }
  layout(size => {
    let whole = screen(caption: cap, raw(src, block: true))
    if measure(block(width: size.width, whole)).height <= _listing_max {
      whole
    } else {
      _long_listing(cap, src)
    }
  })
}

// A piece of a source file from code/, cut from the file itself so that it
// cannot drift from the program that was run. `from` is how one of its lines
// begins. With no `until`, the piece is the block of lines around that one,
// from blank line to blank line: a definition and the comment above it. With
// `until`, it runs from the `from` line to the line before the first later
// one that begins with `until`; with `rest: true`, to the end of the file.
// Blank lines and bare `//` lines at either end are dropped. A piece that
// cannot be found stops the build.
#let piece(name, from, until: none, rest: false, caption: "") = {
  let lines = read("code/" + name).split("\n")
  let a = lines.position(l => l.starts-with(from))
  assert(a != none, message: "piece: no line of " + name + " begins with " + from)
  let b = a + 1
  if rest {
    b = lines.len()
  } else if until == none {
    while a > 0 and lines.at(a - 1).trim() != "" { a -= 1 }
    while b < lines.len() and lines.at(b).trim() != "" { b += 1 }
  } else {
    while b < lines.len() and not lines.at(b).starts-with(until) { b += 1 }
    assert(b < lines.len(), message: "piece: no later line of " + name + " begins with " + until)
  }
  let idle(l) = l.trim() == "" or l.trim() == "//"
  while a < b and idle(lines.at(a)) { a += 1 }
  while b > a and idle(lines.at(b - 1)) { b -= 1 }
  screen(caption: if caption == "" { name } else { caption },
    raw(lines.slice(a, b).join("\n"), block: true))
}

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

// ── Plates — the engravings of ILLUSTRATIONS.txt, in assets/ ────────
#let plate(file, caption, width: 62%) = {
  v(3mm)
  block(breakable: false, width: 100%, align(center, {
    image("assets/" + file, width: width)
    v(1.6mm)
    text(font: body_family, style: "italic", size: 9pt, fill: ink_gray, caption)
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
