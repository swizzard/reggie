# reggie

[i](https://swizzard.pizza "sam raker") love [regular expressions](https://en.wikipedia.org/wiki/Regular_expression "wikipedia entry for 'regular expression'"). [other people](https://xkcd.com/1171/ "xkcd slandering regular expressions") do not. 

regexes are far more useful than people give them credit for. want to convert swathes of raw text into something structured enough to be useful? reach for regexes!

however, regexes have [a reputation for being difficult](https://regex.info/blog/2006-09-15/247) that may or may not be deserved. the [dsl](https://en.wikipedia.org/wiki/Domain-specific_language) used can seem arcane and isn't very flexible or composible.

the ultimate goal is rust & python libraries to let users build and manipulate regular expressions that are [compatible with python's regex dsl](https://docs.python.org/3/library/re.html#regular-expression-syntax).

we're _not_ implementing an actual regex engine; `reggie`s outputs will need to be passed into [`re.compile`](https://docs.python.org/3/library/re.html#re.compile) v.s.

## progress
for what i hope are obvious reasons the mvp is ascii-only.

- [ ] parser
  - [x] literals
  - [ ] zero-width literals
    - [ ] `\A`
    - [ ] `\b`
    - [ ] `\B`
    - [ ] `\z`/`\Z`
  - [ ] backref
  - [x] character ranges
  - [ ] alternation
  - [ ] groups
    - [ ] \(inline\) flags
    - [ ] named
    - [ ] named backref
    - [ ] atomic
    - [ ] positive/negative lookahead/-behind
    - [ ] ternary
- [ ] functionality
  - [ ] `(&self..._with_...-> Self` methods in addition to `&mut self`
  - [ ] regex as a whole
    - [ ] add/delete/change flags
    - [ ] stringify
    - [ ] finiteness/length
  - [ ] all components
    - [ ] stringify
    - [ ] finiteness/length
    - [ ] add/delete/get quantifier
  - [ ] literal
    - [ ] mutate content
  - [ ] range
    - [ ] add/delete subranges
    - [ ] complement
  - [ ] group
    - [ ] get by index
    - [ ] get by name
    - [ ] add/delete/change name
    - [ ] add/delete/get flags
    - [ ] mutate content
