use crate::{
    components::{
        CClass, Quantifier, alternatives::Alternatives, element::ZeroWidthLiteral, flags::Flags,
        groups::GroupExt, quantified::Quantified,
    },
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};

#[derive(Clone, Debug)]
pub enum Pattern {
    Pat(Pat),
    Sub(SubPattern),
}

impl Pattern {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        Ok(Self::Pat(Pat::from_pair(pair)?))
    }
    pub fn new_group(
        components: Vec<Self>,
        flags: Option<Flags>,
        name: Option<String>,
        ext: Option<GroupExt>,
    ) -> Self {
        Self::Sub(SubPattern::group_from_subpatterns(
            components.iter().map(Self::into_subpattern).collect(),
            flags,
            name,
            ext,
        ))
    }
    pub fn new_char_set(ranges: Vec<(char, char)>, quantifier: Option<Quantifier>) -> Result<Self> {
        Ok(Self::Sub(SubPattern::new_char_set(ranges, quantifier)?))
    }
    pub fn new_char_class(cc: CClass, quantifier: Option<Quantifier>) -> Self {
        Self::Sub(SubPattern::new_char_class(cc, quantifier))
    }
    pub fn new_literal(lit: String, quantifier: Option<Quantifier>) -> Self {
        Self::Sub(SubPattern::new_literal(lit, quantifier))
    }
    pub fn new_alts(components: Vec<Self>) -> Self {
        Self::Sub(SubPattern::new_alternatives(
            components.iter().map(Pattern::into_subpattern).collect(),
        ))
    }
    pub fn into_group(&self) -> Self {
        match self {
            Self::Pat(Pat {
                flags,
                sub_patterns,
            }) => Self::Sub(SubPattern::group_from_subpatterns(
                sub_patterns.clone(),
                Some(flags.clone()),
                None,
                None,
            )),
            Self::Sub(sp) => Self::Sub(sp.as_group()),
        }
    }
    fn into_subpattern(&self) -> SubPattern {
        let Self::Sub(s) = self.into_group() else {
            unreachable!()
        };
        s
    }
    pub fn quantify(&self, quantifier: Quantifier) -> Self {
        if let SubPattern::Quantified(mut q) = self.into_subpattern() {
            q.quantifier = Some(quantifier);
            Self::Sub(SubPattern::Quantified(q))
        } else {
            unreachable!()
        }
    }
    pub fn alternate(&self, other: &Self) -> Self {
        let l = self.into_subpattern();
        let r = other.into_subpattern();
        Self::Sub(SubPattern::new_alternatives(vec![l, r]))
    }
}

#[derive(Clone, Debug)]
pub struct Pat {
    flags: Flags,
    pub sub_patterns: Vec<SubPattern>,
}

impl Pat {
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
    pub(crate) fn sub_patterns_count(&self) -> usize {
        self.sub_patterns.len()
    }
    pub(crate) fn sub_patterns(&self) -> impl std::iter::Iterator<Item = &SubPattern> {
        self.sub_patterns.iter()
    }
    fn as_string(&self) -> String {
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
    fn flags(&self) -> Flags {
        self.flags.clone()
    }
    fn indexed(&self) -> bool {
        true
    }
    fn is_finite(&self) -> bool {
        for sp in self.sub_patterns.iter() {
            if !sp.is_finite() {
                return false;
            }
        }
        true
    }
    fn min_match_len(&self) -> usize {
        self.sub_patterns.iter().map(|sp| sp.min_match_len()).sum()
    }
}

#[derive(Clone, Debug)]
pub enum SubPattern {
    Alternatives(Alternatives),
    Quantified(Quantified),
    ZeroWidthLiteral(ZeroWidthLiteral),
    Comment(String),
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
    fn group_from_subpatterns(
        components: Vec<Self>,
        flags: Option<Flags>,
        name: Option<String>,
        ext: Option<GroupExt>,
    ) -> Self {
        Self::Quantified(Quantified::subpatterns_to_group(
            components, flags, name, ext,
        ))
    }
    fn new_alternatives(components: Vec<SubPattern>) -> Self {
        Self::Alternatives(Alternatives::from_components(components))
    }
    fn new_char_set(ranges: Vec<(char, char)>, quantifier: Option<Quantifier>) -> Result<Self> {
        Ok(Self::Quantified(Quantified::new_char_set_from_ranges(
            ranges, quantifier,
        )?))
    }
    fn new_char_class(cc: CClass, quantifier: Option<Quantifier>) -> Self {
        Self::Quantified(Quantified::new_char_class(cc, quantifier))
    }
    fn new_literal(lit: String, quantifier: Option<Quantifier>) -> Self {
        Self::Quantified(Quantified::new_literal(lit, quantifier))
    }
    pub fn as_group(&self) -> Self {
        Self::Quantified(Quantified::subpatterns_to_group(
            vec![self.clone()],
            None,
            None,
            None,
        ))
    }
    pub fn flags(&self) -> Flags {
        Flags::empty()
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
    pub fn as_string(&self) -> String {
        match self {
            Self::Alternatives(alts) => alts.as_string(),
            Self::Quantified(quantified) => quantified.as_string(),
            Self::ZeroWidthLiteral(zwl) => zwl.as_string(),
            Self::Comment(c) => format!("(?#{})", c),
        }
    }
    pub fn indexed(&self) -> bool {
        false
    }
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Alternatives(alts) => alts.is_finite(),
            Self::Quantified(quantified) => quantified.is_finite(),
            _ => true,
        }
    }
    pub fn min_match_len(&self) -> usize {
        match self {
            Self::Alternatives(alts) => alts.min_match_len(),
            Self::Quantified(quantified) => quantified.min_match_len(),
            Self::ZeroWidthLiteral(_) => 0,
            Self::Comment(_) => 0,
        }
    }
}
