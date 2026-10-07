#import "style.typ": *
#import "headings.typ": apply-heading-styles, apply-outline-styles
#import "figures.typ": apply-figure-styles

#let tum-thesis-style = {
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
  set par(
    leading: body-leading,
    spacing: body-par-spacing,
    first-line-indent: 0pt,
    justify: true,
  )
  show math.equation: set text(font: body-font, weight: 400)

  show bibliography: set heading(
    outlined: true,
    bookmarked: true,
    numbering: none,
  )

  set heading(numbering: none, bookmarked: true, outlined: true)
  show heading.where(level: 1): set text(
    font: body-font,
    size: 17pt,
    weight: "bold",
  )
  show heading.where(level: 2): set text(
    font: body-font,
    size: body-size,
    weight: "bold",
  )
  show heading.where(level: 3): set text(
    font: body-font,
    size: body-size,
    weight: "bold",
    style: "italic",
  )
  show heading.where(level: 1): set block(above: 12pt, below: 0pt)
  show heading.where(level: 2): set block(above: 12pt, below: 0pt)
  show heading.where(level: 3): set block(above: 12pt, below: 0pt)
  show heading.where(level: 4): it => {
    box(width: 100%, inset: (left: 1.27cm))[
      #text(weight: "bold")[#it.body.]
      #v(-1.2em)
    ]
  }
  show figure.where(kind: table): set figure.caption(position: top)
  show figure.caption: set text(size: 10pt)
  show figure.caption: it => {
    let supplement = it.supplement
    let number = context {
      let kind = it.element.kind
      let ctr = counter(figure.where(kind: kind))
      ctr.display(it.numbering, at: it.element.location())
    }
    block(width: 100%, spacing: 15pt)[
      #align(left)[
        #text(weight: "bold")[#supplement #number]
        #linebreak()
        #text(style: "italic")[#it.body]
      ]
    ]
  }
  show outline.entry.where(level: 1): set text(weight: "bold")
  show outline.entry.where(level: 1): set block(above: 2em, below: 0pt)
  show outline.entry.where(level: 2): set block(above: 0.5em, below: 0pt)
}
