#import "body.typ": *

#let acknowledgements(body) = {
  text(size: 17pt, weight: "bold")[Acknowledgements]
  v(1.5em)
  set text(font: body-font, size: body-size, lang: "en")
  v(6cm)
  align(center)[#body]
}