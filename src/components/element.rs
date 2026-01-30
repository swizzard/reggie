use crate::{
    components::{char_set::CharSet, traits::AsComponent},
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::Pair;
#[derive(Clone, Debug)]
pub enum Element {
    CharSet(CharSet),
    Literal(Literal),
}

impl Element {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::char_set => Ok(Self::CharSet(CharSet::from_pair(pair)?)),
            Rule::literals => Ok(Self::Literal(Literal::from_pair(pair)?)),
            _ => Err(ReggieError::unexpected_input(pair).into()),
        }
    }
    pub fn charset_from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::CharSet(CharSet::from_pair(pair)?))
    }
    pub fn literals_from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::Literal(Literal::from_pair(pair)?))
    }
}

impl AsComponent for Element {
    fn as_string(&self) -> String {
        match self {
            Self::CharSet(cs) => cs.as_string(),
            Self::Literal(l) => l.as_string(),
        }
    }
    fn min_match_len(&self) -> usize {
        match self {
            Self::CharSet(cs) => cs.min_match_len(),
            Self::Literal(l) => l.min_match_len(),
        }
    }
    fn is_finite(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct Literal(String);

impl Literal {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let r = pair.as_rule();
        if let Rule::literals = r {
            Ok(Self(String::from(pair.as_str())))
        } else {
            Err(ReggieError::unexpected_input(pair).into())
        }
    }
    pub fn as_string(&self) -> String {
        self.0.clone()
    }
    pub fn min_match_len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ZeroWidthLiteral {
    InputStart,
    InputEnd,
    WordBoundary,
    NotWordBoundary,
}

impl ZeroWidthLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let s = pair.as_str();
        match s {
            "\\A" | "\\a" => Ok(Self::InputStart),
            "\\b" => Ok(Self::WordBoundary),
            "\\B" => Ok(Self::NotWordBoundary),
            "\\Z" | "\\z" => Ok(Self::InputEnd),
            _ => Err(ReggieError::InvalidLiteral {
                bad_literal: s.into(),
            }
            .into()),
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::InputStart => String::from("\\a"),
            Self::InputEnd => String::from("\\z"),
            Self::NotWordBoundary => String::from("\\B"),
            Self::WordBoundary => String::from("\\b"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_literal_min_match_len() {
        let l = Literal("foo".into());
        assert_eq!(3, l.min_match_len())
    }
    #[test]
    fn test_literal_as_string() {
        let foo: String = "foo".into();
        let l = Literal(foo.clone());
        assert_eq!(foo, l.as_string());
    }
    #[test]
    fn test_zwl_as_string() {
        assert_eq!(
            String::from("\\a"),
            ZeroWidthLiteral::InputStart.as_string()
        );
        assert_eq!(String::from("\\z"), ZeroWidthLiteral::InputEnd.as_string());
        assert_eq!(
            String::from("\\B"),
            ZeroWidthLiteral::NotWordBoundary.as_string()
        );
        assert_eq!(
            String::from("\\b"),
            ZeroWidthLiteral::WordBoundary.as_string()
        );
    }
}
