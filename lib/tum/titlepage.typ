#import "body.typ": *

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
        image("figures/Universitaet_Logo_RGB.pdf", height: tum-logo-height),
      )
    ],
    header-ascent: tum-logo-height + 1cm,
    footer: none,
    numbering: none,
    foreground: place(
      bottom + right,
      dx: -2 * tum-logo-height,
      dy: -tum-logo-height,
      image("figures/TUM_Uhrenturm.png", width: 11cm),
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