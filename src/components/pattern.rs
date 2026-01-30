use crate::{
    components::{
        alternatives::Alternatives,
        element::{Element, ZeroWidthLiteral},
        flags::Flags,
        groups::Group,
        quantified::Quantified,
    },
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};

#[derive(Clone, Debug)]
pub struct Pattern {
    flags: Flags,
    pub sub_patterns: Vec<SubPattern>,
}

impl Pattern {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let mut flags = Flags::empty();
        let mut sub_patterns = Vec::new();
        while let Some(matched) = inner.next() {
            match matched.as_rule() {
                Rule::sub_pattern => sub_patterns.push(SubPattern::from_pair(matched)?),
                Rule::whole_pattern_flags => {
                    let mut parsed_flags = Flags::from_whole_pattern_pair(matched)?;
                    std::mem::swap(&mut flags, &mut parsed_flags);
                }
                _ => return Err(ReggieError::unexpected_input(matched).into()),
            }
        }
        Ok(Self {
            flags,
            sub_patterns,
        })
    }
    pub fn flags(&self) -> &Flags {
        &self.flags
    }
    pub fn as_string(&self) -> String {
        let mut s = if self.flags.is_empty() {
            String::new()
        } else {
            format!("({})", self.flags.as_string())
        };
        for sp in self.sub_patterns.iter() {
            s.push_str(sp.as_string().as_str())
        }
        s
    }
    pub fn is_finite(&self) -> bool {
        for sp in self.sub_patterns.iter() {
            if !sp.is_finite() {
                return false;
            }
        }
        true
    }
    // TODO(SHR): fix these
    pub fn min_match_len(&self) -> usize {
        self.sub_patterns.iter().map(|sp| sp.min_match_len()).sum()
    }
    pub(crate) fn sub_patterns_count(&self) -> usize {
        self.sub_patterns.len()
    }
    pub(crate) fn sub_patterns(&self) -> impl std::iter::Iterator<Item = &SubPattern> {
        self.sub_patterns.iter()
    }
}

#[derive(Clone, Debug)]
pub enum Quantifiable {
    Element(Element),
    Group(Group),
}

#[derive(Clone, Debug)]
pub enum SubPattern {
    Alternatives(Alternatives),
    Quantified(Quantified),
    ZeroWidthLiteral(ZeroWidthLiteral),
    Comment(String),
    Group(Group),
}

impl SubPattern {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let mut inner = pair.into_inner();
        if let Some(p) = inner.next() {
            SubPattern::single_from_pair(p, &mut inner)
        } else {
            Err(ReggieError::unexpected_eoi(char_ix).into())
        }
    }
    pub fn single_from_pair(pair: Pair<Rule>, inner: &mut Pairs<'_, Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::alternatives => SubPattern::alternatives_from_pair(pair),
            Rule::group | Rule::literals | Rule::char_set => {
                SubPattern::quantified_from_pair(pair, inner)
            }
            Rule::zero_width_literal => SubPattern::zwl_from_pair(pair),
            Rule::comment_group => SubPattern::comment_group_from_pair(pair),
            _ => {
                println!("single_from_pair actually {:?}", pair.as_rule());
                Err(ReggieError::unexpected_input(pair).into())
            }
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::Alternatives(alts) => alts.as_string(),
            Self::Quantified(quantified) => quantified.as_string(),
            Self::ZeroWidthLiteral(zwl) => zwl.as_string(),
            Self::Comment(c) => format!("(?#{})", c),
            Self::Group(g) => g.as_string(),
        }
    }
    pub fn min_match_len(&self) -> usize {
        match self {
            Self::Alternatives(alts) => alts.min_match_len(),
            Self::Quantified(quantified) => quantified.min_match_len(),
            // { el, quantifier } => {
            //     el.min_match_len() * quantifier.map(|q| q.min_len_multiplier()).unwrap_or(1)
            // }
            Self::ZeroWidthLiteral(_) => 0,
            Self::Comment(_) => 0,
            Self::Group(g) => g.min_match_len(),
        }
    }
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Alternatives(alts) => alts.is_finite(),
            Self::Quantified(quantified) => quantified.is_finite(),
            // Self::Quantifiable { quantifier, .. } => {
            //     quantifier.map(|q| q.is_finite()).unwrap_or(true)
            // }
            Self::Group(g) => g.is_finite(),
            _ => true,
        }
    }
    pub fn flags(&self) -> Option<Flags> {
        match self {
            SubPattern::Group(g) => g.flags(),
            _ => None,
        }
    }
    pub(crate) fn inner_components(inner: Pairs<'_, Rule>) -> Result<Vec<Self>> {
        let mut comps: Vec<Self> = Vec::new();
        for p in inner {
            match p.as_rule() {
                Rule::sub_pattern => comps.push(Self::from_pair(p)?),
                Rule::r_parens => continue,
                _ => {
                    return Err(ReggieError::unexpected_input(p).into());
                }
            }
        }
        Ok(comps)
    }

    fn alternatives_from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::Alternatives(Alternatives::from_pair(pair)?))
    }
    fn quantified_from_pair(pair: Pair<Rule>, inner: &mut Pairs<'_, Rule>) -> Result<Self> {
        Ok(Self::Quantified(Quantified::from_pair(pair, inner)?))
    }
    fn zwl_from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::ZeroWidthLiteral(ZeroWidthLiteral::from_pair(pair)?))
    }
    fn comment_group_from_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let inner = pair.into_inner();
        let content = inner
            .skip(3)
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?; // (?#
        Ok(Self::Comment(content.as_str().into()))
    }
    fn group_from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::Group(Group::from_pair(pair)?))
    }
    fn ext_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Result<Self> {
        Ok(Self::Group(Group::ext_group_from_pairs(fst, inner)?))
    }
    fn plain_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Result<Self> {
        Ok(Self::Group(Group::plain_group_from_pairs(fst, inner)?))
    }
}
