#import "body.typ": *

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