use crate::{error::ReggieError, parser::Rule};
use anyhow::Result;
use disjoint_ranges::{DisjointRange, UnaryRange};
use pest::iterators::Pair;
#[derive(Clone, Debug)]
pub struct CharSet {
    char_ranges: DisjointRange<char>,
}

impl CharSet {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let r = pair.as_rule();
        if let Rule::char_set = r {
            let mut char_ranges = DisjointRange::empty();
            let mut negated = false;
            let mut pairs_iter = pair.into_inner();
            while let Some(p) = pairs_iter.next() {
                match p.as_rule() {
                    Rule::set_negation => negated = true,
                    Rule::char_range => {
                        let (_, char_ix) = p.line_col();
                        let mut inner = p.into_inner();
                        let low = inner
                            .next()
                            .ok_or(ReggieError::unexpected_eoi(char_ix))?
                            .as_str()
                            .chars()
                            .nth(0)
                            .ok_or(ReggieError::unexpected_eoi(char_ix))?;
                        inner.next();
                        let high = inner
                            .next()
                            .ok_or(ReggieError::unexpected_eoi(char_ix))?
                            .as_str()
                            .chars()
                            .nth(0)
                            .ok_or(ReggieError::unexpected_eoi(char_ix))?;
                        char_ranges.add_unary_range(UnaryRange::new_unchecked(low, high));
                    }
                    Rule::hyphen => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('-', '-'))
                    }
                    Rule::set_literal => {
                        let c = p
                            .as_str()
                            .chars()
                            .nth(0)
                            .ok_or(ReggieError::unexpected_eoi(p.line_col().1))?;
                        char_ranges.add_unary_range(UnaryRange::new_unchecked(c, c));
                    }
                    Rule::escaped_hyphen => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('-', '-'));
                    }
                    Rule::caret => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('^', '^'));
                    }
                    Rule::char_class => {
                        let cls = CharClass::from_pair(p)?;
                        char_ranges.add_disjoint_range(cls.to_range());
                    }
                    Rule::l_sq | Rule::r_sq => continue,
                    _ => return Err(ReggieError::unexpected_input(p).into()),
                };
            }
            if negated {
                Ok(Self {
                    char_ranges: char_ranges.complement(),
                })
            } else {
                Ok(Self { char_ranges })
            }
        } else {
            println!("actually {:?}", r);
            unreachable!()
        }
    }
    pub(crate) fn as_string(&self) -> String {
        let mut s = String::from("[");
        for subrange in self.char_ranges.ranges_iter() {
            let (low, high) = subrange.as_bounds();
            s.push_str(format!("{}-{}", low, high).as_str());
        }
        s.push_str("]");
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CClass {
    D,
    S,
    W,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharClass {
    class: CClass,
    negated: bool,
}

impl CharClass {
    pub fn to_range(&self) -> DisjointRange<char> {
        let range = match self.class {
            CClass::D => CharClass::digit_range(),
            CClass::S => CharClass::whitespace_range(),
            CClass::W => CharClass::word_range(),
        };
        if self.negated {
            range.complement()
        } else {
            range
        }
    }
    fn digit_range() -> DisjointRange<char> {
        DisjointRange::new_single_range_unchecked('0', '9')
    }
    fn whitespace_range() -> DisjointRange<char> {
        DisjointRange::from_bounds_unchecked([('\t', '\r'), (' ', ' ')])
    }
    fn word_range() -> DisjointRange<char> {
        DisjointRange::from_bounds_unchecked([('a', 'z'), ('A', 'Z'), ('0', '9')])
    }
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let mut inner = pair.into_inner();
        inner.next(); // backslash
        let c = inner
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .as_str();
        Ok(match c {
            "d" => Self {
                class: CClass::D,
                negated: false,
            },
            "D" => Self {
                class: CClass::D,
                negated: true,
            },
            "s" => Self {
                class: CClass::S,
                negated: false,
            },
            "S" => Self {
                class: CClass::S,
                negated: true,
            },
            "w" => Self {
                class: CClass::W,
                negated: false,
            },
            "W" => Self {
                class: CClass::W,
                negated: true,
            },
            _ => {
                println!("c {:?}", c);
                unreachable!()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_set_as_string() {
        let cs = CharSet {
            char_ranges: DisjointRange::from_bounds_unchecked([('a', 'c'), ('e', 'g')]),
        };
        let expected = String::from("[a-ce-g]");
        assert_eq!(expected, cs.as_string())
    }
}
