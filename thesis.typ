#import "content/metadata.typ": metadata
#import "content/content.typ": content
#import "lib/tum/headings.typ": *
#import "lib/tum/figures.typ": *
#import "lib/tum/elements.typ": *
#import "lib/tum/titlepage.typ": *
#import "lib/tum/abstract.typ": *
#import "lib/tum/acknowledgements.typ": *
#import "lib/tum/declaration.typ": *

#let mark-errors(it) = {
  let pattern = regex("!\{([^}]+)\}")
  let raw-text = it.text
  let last-pos = 0
  let elements = ()

  for m in raw-text.matches(pattern) {
    if m.start > last-pos {
      elements.push(raw(raw-text.slice(last-pos, m.start), lang: it.lang, block: false))
    }

    let error-text = m.captures.at(0)
    elements.push(underline(
      stroke: 1pt + red,
      offset: 2pt,
      evade: false,
      raw(error-text, lang: it.lang, block: false)
    ))

    last-pos = m.end
  }

  // 1c. Highlight remaining text after the last error
  if last-pos < raw-text.len() {
    elements.push(raw(raw-text.slice(last-pos), lang: it.lang, block: false))
  }

  align(left, elements.join())
}

#let thesis() = {
  titlepage(metadata)

  pagebreak()
  acknowledgements(content.acknowledgements)

  pagebreak()
  abstract(content.abstract)

  pagebreak()
  outline(
    title: heading(outlined: false, bookmarked: true)[Contents],
    depth: 4,
  )
  outline(
    title: heading(outlined: false, bookmarked: true)[List of Tables],
    target: figure.where(kind: table),
  )
  outline(
    title: heading(outlined: false, bookmarked: true)[List of Figures],
    target: figure.where(kind: image),
  )
  pagebreak()

  set page(
    paper: "a4",
    margin: tum-margin,
    header: context {
      align(right)[#counter(page).display()]
    },
    footer: none,
    numbering: "1",
    number-align: right + top,
  )
  set text(font: body-font, size: body-size, lang: "en")

  show math.equation: set text(font: "TeX Gyre Termes Math", weight: 400)

  show figure: it => {
    it
    // lovelace draws line labels as invisible figures next to the line numbers; they get no space
    if it.kind != "lovelace-line-number" {
      v(1em)
    }
  }

  show figure.caption: set text(size: 10.5pt)

  show raw.where(block: true): it => {
    block(
      fill: luma(230),
      inset: 8pt,
      radius: 4pt,
      mark-errors(it),
    )
  }

  show bibliography: set heading(
    outlined: true,
    bookmarked: true,
    numbering: none,
  )

  set heading(numbering: "1.", bookmarked: true, outlined: true)

  pagebreak(weak: true)
  content.introduction
  pagebreak(weak: true)
  content.related-work
  pagebreak(weak: true)
  content.solution-design
  pagebreak(weak: true)
  content.implementation
  pagebreak(weak: true)
  content.evaluation
  pagebreak(weak: true)
  content.discussion
  pagebreak(weak: true)
  content.conclusion

  pagebreak()
  bibliography("content/bibliography.bib", style: "lib/tum/ieee-thesis.csl")

  pagebreak()
  content.appendix

  pagebreak()
  declaration-of-academic-integrity(metadata)
}
