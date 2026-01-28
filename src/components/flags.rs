use crate::{error::ReggieError, parser::Rule};
use anyhow::Result;
use pest::iterators::Pair;
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    collections::BTreeSet,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flags(BTreeSet<Flag>);

impl Flags {
    pub(crate) fn new() -> Self {
        Self(BTreeSet::new())
    }
    pub(crate) fn add(&mut self, flag: Flag) {
        self.0.insert(flag);
    }
    pub(crate) fn remove(&mut self, flag: &Flag) {
        self.0.remove(flag);
    }
    pub fn as_string(&self) -> String {
        let mut s = String::from("?");
        for flag in self.0.iter() {
            s.push_str(flag.as_str())
        }
        s
    }
    pub(crate) fn from_whole_pattern_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let mut inner = pair.into_inner();
        inner.next(); // (?
        let flag_match = inner.next().ok_or(ReggieError::unexpected_eoi(char_ix))?;
        if flag_match.as_rule() == Rule::flags {
            let mut flags = Flags::new();
            for c in flag_match.as_str().chars() {
                flags.add(Flag::from_char(c)?);
            }
            Ok(flags)
        } else {
            Err(ReggieError::unexpected_input(flag_match).into())
        }
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupFlags {
    pos: Flags,
    neg: Flags,
}

impl GroupFlags {
    pub(crate) fn empty() -> Self {
        Self {
            pos: Flags::new(),
            neg: Flags::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.neg.is_empty()
    }

    pub(crate) fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let mut pos = Flags::new();
        let mut neg = Flags::new();
        let (_, char_ix) = pair.line_col();
        let mut s = pair.as_str().split('-');
        for c in s
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .chars()
        {
            pos.add(Flag::from_char(c)?);
        }
        if let Some(neg_flag_str) = s.next() {
            for c in neg_flag_str.chars() {
                neg.add(Flag::from_char(c)?)
            }
        };
        Ok(Self { pos, neg })
    }
    pub fn as_string(&self) -> String {
        if !self.neg.0.is_empty() {
            format!("{}-{}", self.pos.as_string(), self.neg.as_string())
        } else {
            self.pos.as_string()
        }
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

    // #[test]
    // fn test_flags_as_string() {
    //     let flags = Flags(HashSet::from([
    //         Flag::Ignorecase,
    //         Flag::Multiline,
    //         Flag::Dotall,
    //     ]));
    //     let expected = String::from("?msi");
    //     assert_eq!(expected, flags.as_string())
    // }
}
