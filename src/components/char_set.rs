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
    pub(crate) fn from_ranges(ranges: Vec<(char, char)>) -> Result<Self> {
        Ok(Self {
            char_ranges: DisjointRange::from_bounds(ranges.clone())
                .ok_or(ReggieError::InvalidRanges { bad_ranges: ranges })?,
        })
    }
    pub(crate) fn from_cclass(cclass: CClass) -> Self {
        Self {
            char_ranges: cclass.to_char_class().to_range(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CClass {
    D,
    S,
    W,
    NegD,
    NegS,
    NegW,
}

impl CClass {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.strip_prefix("\\").ok_or(ReggieError::InvalidCharClass {
            bad_cclass: String::from(s),
        })? {
            "d" => Ok(Self::D),
            "D" => Ok(Self::NegD),
            "s" => Ok(Self::S),
            "S" => Ok(Self::NegS),
            "w" => Ok(Self::W),
            "W" => Ok(Self::NegW),
            other => Err(ReggieError::InvalidCharClass {
                bad_cclass: String::from(other),
            }
            .into()),
        }
    }
    pub(crate) fn to_char_class(self) -> CharClass {
        match self {
            Self::D => CharClass {
                class: CC::D,
                negated: false,
            },
            Self::NegD => CharClass {
                class: CC::D,
                negated: true,
            },
            Self::S => CharClass {
                class: CC::S,
                negated: false,
            },
            Self::NegS => CharClass {
                class: CC::S,
                negated: true,
            },
            Self::W => CharClass {
                class: CC::W,
                negated: false,
            },
            Self::NegW => CharClass {
                class: CC::W,
                negated: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CC {
    D,
    S,
    W,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharClass {
    class: CC,
    negated: bool,
}

impl CharClass {
    pub fn to_range(&self) -> DisjointRange<char> {
        let range = match self.class {
            CC::D => CharClass::digit_range(),
            CC::S => CharClass::whitespace_range(),
            CC::W => CharClass::word_range(),
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
        Ok(CClass::from_str(c)?.to_char_class())
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
