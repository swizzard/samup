# samup

`samup` is a a wee markup language in the form of code that transforms text written in that language into html

please see [the grammar](./grammar.md) for a hopefully-helpful formal representation or the [integration tests](./tests/integration.rs) for examples

## name

it's called `samup` because i'm sam not mark and it's up not down and lbr the `__ml` kicky name tar sands are just about fracked out

## features/roadmap

- [x] `<i>` and `<strong>`
- [x] links (but see below)
- [x] footnotes (but see below)
- [x] `<h1>`-`<h6>`
- [ ] escaping
  - [ ] code blocks?
- [ ] lists
- [ ] cli/io

## differences from markdown

- anything besides `#+` and `\[\^\d+\]:` gets wrapped in `<p>...</p>`
  - `#+` becomes `<h_>`
  - `\[\^\d+\]:` becomes a footnote reference (see more below)
- link syntax
  - `[url]` becomes `<a href="url" target="_blank">url</a>`
  - `[url](label)` becomes `<a href="url" target="_blank">label</a>`
- footnotes
  - foot note references are rendered _in-place_, and not automatically moved to the end of the output
