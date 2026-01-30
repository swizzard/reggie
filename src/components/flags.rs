use crate::{error::ReggieError, parser::Rule};
use anyhow::Result;
use pest::iterators::Pair;
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    collections::BTreeSet,
    fmt::Write,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flags {
    pos: BTreeSet<Flag>,
    neg: BTreeSet<Flag>,
}

impl Flags {
    pub(crate) fn empty() -> Self {
        Self {
            pos: BTreeSet::new(),
            neg: BTreeSet::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.neg.is_empty()
    }

    pub(crate) fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let mut pos = BTreeSet::new();
        let mut neg = BTreeSet::new();
        let (_, char_ix) = pair.line_col();
        let mut s = pair.as_str().split('-');
        for c in s
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .chars()
        {
            pos.insert(Flag::from_char(c)?);
        }
        if let Some(neg_flag_str) = s.next() {
            for c in neg_flag_str.chars() {
                neg.insert(Flag::from_char(c)?);
            }
        };
        Ok(Self { pos, neg })
    }
    pub(crate) fn from_whole_pattern_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let mut inner = pair.into_inner();
        inner.next(); // (?
        let flag_match = inner.next().ok_or(ReggieError::unexpected_eoi(char_ix))?;
        if flag_match.as_rule() == Rule::flags {
            let mut flags = BTreeSet::new();
            for c in flag_match.as_str().chars() {
                flags.insert(Flag::from_char(c)?);
            }
            Ok(Self {
                pos: flags,
                neg: BTreeSet::new(),
            })
        } else {
            Err(ReggieError::unexpected_input(flag_match).into())
        }
    }
    pub(crate) fn as_string(&self) -> String {
        let mut s = format!(
            "?{}",
            self.pos.iter().map(|f| f.as_str()).collect::<String>()
        );
        if !self.neg.is_empty() {
            write!(
                s,
                "-{}",
                self.neg.iter().map(|f| f.as_str()).collect::<String>()
            )
            .unwrap();
        }
        s
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Flag {
    Ascii,
    Ignorecase,
    Locale,
    Multiline,
    Dotall,
    Unicode,
    Verbose,
}

impl PartialOrd for Flag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Flag {
    fn cmp(&self, other: &Self) -> Ordering {
        use Flag::*;
        match (self, other) {
            (Ascii, Ascii) => Ordering::Equal,
            (Ascii, _) => Ordering::Greater,
            (Ignorecase, Ignorecase) => Ordering::Equal,
            (Ignorecase, _) => Ordering::Greater,
            (Locale, Locale) => Ordering::Equal,
            (Locale, _) => Ordering::Equal,
            (Multiline, Multiline) => Ordering::Equal,
            (Multiline, _) => Ordering::Equal,
            (Dotall, Dotall) => Ordering::Equal,
            (Dotall, _) => Ordering::Equal,
            (Unicode, Unicode) => Ordering::Equal,
            (Unicode, _) => Ordering::Equal,
            (Verbose, Verbose) => Ordering::Equal,
            (Verbose, _) => Ordering::Equal,
        }
    }
}

impl Flag {
    fn as_str(&self) -> &'static str {
        match self {
            Flag::Ascii => "a",
            Flag::Ignorecase => "i",
            Flag::Locale => "L",
            Flag::Multiline => "m",
            Flag::Dotall => "s",
            Flag::Unicode => "u",
            Flag::Verbose => "x",
        }
    }
    pub fn from_char(c: char) -> Result<Self> {
        match c {
            'a' => Ok(Self::Ascii),
            'i' => Ok(Self::Ignorecase),
            'L' => Ok(Self::Locale),
            'm' => Ok(Self::Multiline),
            's' => Ok(Self::Dotall),
            'u' => Ok(Self::Unicode),
            'x' => Ok(Self::Verbose),
            _ => Err(ReggieError::InvalidFlag { bad_flag: c }.into()),
        }
    }
    pub fn as_string(&self) -> String {
        String::from(self.as_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_group_flags_as_string() {
        let flags = Flags {
            pos: BTreeSet::from([Flag::Ignorecase, Flag::Multiline]),
            neg: BTreeSet::from([Flag::Dotall]),
        };
        let expected = String::from("?im-s");
        assert_eq!(expected, flags.as_string())
    }
}
