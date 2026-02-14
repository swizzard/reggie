use crate::{
    components::{
        CClass, Quantifier,
        alternatives::Alternatives,
        element::ZeroWidthLiteral,
        flags::{Flag, Flags},
        groups::GroupExt,
        quantified::Quantified,
    },
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};
use std::fmt::Write;

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
    pub fn new_character_set(
        ranges: Vec<(char, char)>,
        quantifier: Option<Quantifier>,
    ) -> Result<Self> {
        Ok(Self::Sub(SubPattern::new_char_set(ranges, quantifier)?))
    }
    pub fn new_character_class(cc: CClass, quantifier: Option<Quantifier>) -> Self {
        Self::Sub(SubPattern::new_char_class(cc, quantifier))
    }
    pub fn new_literal(lit: String, quantifier: Option<Quantifier>) -> Self {
        Self::Sub(SubPattern::new_literal(lit, quantifier))
    }
    pub fn new_alternatives(components: Vec<Self>) -> Self {
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
    pub fn quantify(&self, quantifier: Quantifier) -> Self {
        if let SubPattern::Quantified(mut q) = self.into_subpattern() {
            q.quantifier = Some(quantifier);
            Self::Sub(SubPattern::Quantified(q))
        } else {
            unreachable!()
        }
    }
    pub fn alternate_with(&self, other: &Self) -> Self {
        let l = self.into_subpattern();
        let r = other.into_subpattern();
        Self::Sub(SubPattern::new_alternatives(vec![l, r]))
    }
    pub fn follow_with(&self, other: &Self) -> Self {
        Self::new_group(vec![self.clone(), other.clone()], None, None, None)
    }
    pub fn with_flags(&self, flags: Flags) -> Result<Self> {
        if flags.has_neg() {
            Err(ReggieError::NegativePatternFlags.into())
        } else {
            Ok(match self {
                Self::Sub(sp) => Self::Pat(Pat {
                    flags,
                    sub_patterns: vec![sp.clone()],
                }),
                Self::Pat(Pat { sub_patterns, .. }) => Self::Pat(Pat {
                    flags,
                    sub_patterns: sub_patterns.clone(),
                }),
            })
        }
    }
    pub fn with_flag(&self, flag: Flag) -> Self {
        match self {
            Self::Sub(sp) => Self::Pat(Pat {
                flags: Flags::new_single(flag),
                sub_patterns: vec![sp.clone()],
            }),
            Self::Pat(Pat {
                flags,
                sub_patterns,
            }) => {
                let new_flags = flags.add_flag(flag);
                Self::Pat(Pat {
                    flags: new_flags,
                    sub_patterns: sub_patterns.clone(),
                })
            }
        }
    }
    pub fn without_flag(&self, flag: Flag) -> Self {
        match self {
            Self::Sub(sp) => sp.without_flag(flag),
            Self::Pat(p) => p.without_flag(flag),
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::Pat(p) => p.as_string(),
            Self::Sub(sp) => sp.as_string(),
        }
    }
    pub fn min_match_len(&self) -> usize {
        match self {
            Self::Pat(p) => p.min_match_len(),
            Self::Sub(s) => s.min_match_len(),
        }
    }
    pub fn is_finite(&self) -> bool {
        match &self {
            Self::Sub(sp) => sp.is_finite(),
            Self::Pat(p) => p.is_finite(),
        }
    }
    pub fn groups_count(&self) -> usize {
        match self {
            Self::Pat(Pat { sub_patterns, .. }) => {
                sub_patterns.iter().map(SubPattern::groups_count).sum()
            }
            Self::Sub(sp) => sp.groups_count(),
        }
    }
    pub fn flags(&self) -> Option<Flags> {
        match &self {
            Self::Pat(Pat { flags, .. }) => Some(flags.clone()),
            Self::Sub(_) => None,
        }
    }
    pub fn nth_group(&self, n: usize) -> Option<Self> {
        if n == 0 {
            Some(self.clone())
        } else {
            match self {
                Self::Pat(p) => p.nth_group(n),
                Self::Sub(s) => s.nth_group(n),
            }
        }
    }
    pub fn components(&self) -> Vec<Self> {
        match &self {
            Self::Sub(_) => vec![self.clone()],
            Self::Pat(Pat { sub_patterns, .. }) => {
                sub_patterns.iter().map(SubPattern::as_pattern).collect()
            }
        }
    }
    fn into_subpattern(&self) -> SubPattern {
        let Self::Sub(s) = self.into_group() else {
            unreachable!()
        };
        s
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
    fn nth_group(&self, n: usize) -> Option<Pattern> {
        if n == 0 {
            Some(Pattern::Pat(self.clone()))
        } else {
            let i = n;
            let mut sps = self.sub_patterns.iter();
            while let Some(sp) = sps.next() {
                let ng = sp.nth_group(i);
                if ng.is_some() {
                    return ng;
                }
            }
            None
        }
    }

    fn as_string(&self) -> String {
        let mut s = if self.flags.is_empty() {
            String::new()
        } else {
            format!("({})", self.flags.as_string())
        };
        for sp in self.sub_patterns.iter() {
            write!(&mut s, "{}", sp.as_string()).unwrap();
        }
        s
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
    fn without_flag(&self, flag: Flag) -> Pattern {
        let mut new = self.clone();
        let new_flags = new.flags.remove_flag(flag);
        new.flags = new_flags;
        Pattern::Pat(new)
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
    pub fn groups_count(&self) -> usize {
        match self {
            Self::ZeroWidthLiteral(_) | Self::Comment(_) => 0,
            Self::Alternatives(alts) => alts.groups_count(),
            Self::Quantified(q) => q.groups_count(),
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
    pub fn as_string(&self) -> String {
        match self {
            Self::Alternatives(alts) => alts.as_string(),
            Self::Quantified(quantified) => quantified.as_string(),
            Self::ZeroWidthLiteral(zwl) => zwl.as_string(),
            Self::Comment(c) => format!("(?#{})", c),
        }
    }
    pub(crate) fn nth_group(&self, n: usize) -> Option<Pattern> {
        if n == 0 {
            Some(Pattern::Sub(self.clone()))
        } else {
            match self {
                Self::Alternatives(alts) => alts.nth_group(n),
                Self::Quantified(q) => q.nth_group(n),
                _ => None,
            }
        }
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
    fn without_flag(&self, flag: Flag) -> Pattern {
        match self {
            Self::Quantified(q) => Pattern::Sub(Self::Quantified(q.without_flag(flag))),
            other => Pattern::Sub(self.clone()),
        }
    }

    fn as_pattern(&self) -> Pattern {
        Pattern::Sub(self.clone())
    }
}
