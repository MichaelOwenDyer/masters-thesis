#import "content/metadata.typ": metadata
#import "content/content.typ": content
#import "lib/tum/headings.typ": *
#import "lib/tum/figures.typ": *
#import "lib/tum/body.typ": *
#import "lib/tum/elements.typ": *

#let titlepage(meta) = {
  page(
    paper: "a4",
    margin: tum-margin,
    header: pad(top: 1cm)[
      #set text(font: sans-font, size: 9pt, fill: tum-blau)
      #grid(
        columns: (1fr, auto),
        align: (left + top, right + top),
        box(width: 100%)[
          #meta.chair \
          #meta.department \
          #meta.school \
          #meta.at("university")
        ],
        image("figures/tum/Universitaet_Logo_RGB.pdf", height: tum-logo-height),
      )
    ],
    header-ascent: tum-logo-height + 1cm,
    footer: none,
    numbering: none,
    foreground: place(
      bottom + right,
      dx: -2 * tum-logo-height,
      dy: -tum-logo-height,
      image("figures/tum/TUM_Uhrenturm.png", width: 11cm),
    ),
  )[
    #set text(font: sans-font, size: body-size, lang: "en")
    #set par(leading: 1em, spacing: 0.3cm, first-line-indent: 0pt)

    #v(0.5cm)
    #text(size: 17pt, weight: "bold")[#meta.title]
    #v(0.3cm)
    #text(size: 14pt)[#meta.at("german-title")]
    #v(0.3cm)
    #text(size: 14pt, weight: "bold", fill: tum-blau)[#meta.author]
    #v(1cm)
    #text(size: 14pt)[
      Thesis \
      for the Attainment of the Degree \
      *#meta.degree* \
      #v(body-par-spacing)

      at the #meta.school, \
      #meta.department, \
      #meta.chair
    ]
    #v(1cm)
    #text(size: body-size)[
      *Examiner* \
      #meta.examiner \
      #v(12pt)

      *Supervised by* \
      #meta.supervisor \
      #v(12pt)

      #if meta.supervisor-2 != none [
        *Supervisor 2* \
        #meta.supervisor-2 \
        #v(12pt)
      ]

      *Submitted by* \
      #meta.author \
      #v(12pt)

      *Submitted on* \
      #meta.at("submission-date")
    ]
  ]
}

#let acknowledgements(body) = {
  text(size: 17pt, weight: "bold")[Acknowledgements]
  v(1.5em)
  set text(font: body-font, size: body-size, lang: "en")
  v(6cm)
  align(center)[#body]
}

#let abstract(body) = {
  set page(
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
  apply-heading-styles()

  heading(outlined: true, bookmarked: true)[Abstract]
  body
}

#let declaration-of-academic-integrity(meta) = [
  #set text(font: body-font, size: body-size, lang: "en")
  #set par(
    leading: body-leading,
    spacing: body-par-spacing,
    first-line-indent: 0pt,
    justify: false,
  )

  #align(center)[
    #text(size: 20pt, weight: "bold")[
      Declaration of Academic Integrity
    ]
  ]
  #v(10mm)

  I hereby declare that the thesis submitted is my own unaided work.
  All direct or indirect sources used are acknowledged as references.

  I am aware that the thesis in digital form can be examined for the use of unauthorized aid and in order
  to determine whether the thesis as a whole or parts incorporated in it may be deemed as plagiarism.
  For the comparison of my work with existing sources I agree that it shall be entered in a database
  where it shall also remain after examination, to enable comparison with future theses submitted.
  Further rights of reproduction and usage, however, are not granted here.

  This thesis was not previously presented to another examination board and has not been published.

  #v(30mm)
  #grid(
    columns: (1fr, auto),
    align: (left, right),
    [#meta.submission-place, #meta.submission-date],
    [#meta.author],
  )
]

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

  show bibliography: set heading(
    outlined: true,
    bookmarked: true,
    numbering: none,
  )

  set heading(numbering: "1.", bookmarked: true, outlined: true)

  content.introduction
  content.related-work
  content.solution-design
  content.implementation
  content.evaluation
  content.discussion
  content.conclusion

  pagebreak()
  bibliography("content/bibliography.bib", style: "ieee")

  pagebreak()
  content.appendix

  pagebreak()
  declaration-of-academic-integrity(metadata)
}
