use crate::{
    components::{flags::Flags, pattern::SubPattern},
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};
use std::fmt::Write;
#[derive(Clone, Debug, PartialEq)]
pub enum GroupExt {
    NonCapturing,
    Atomic,
    PosLookahead,
    NegLookahead,
    PosLookbehind,
    NegLookbehind,
}

impl GroupExt {
    pub fn as_string(&self) -> String {
        match self {
            Self::NonCapturing => String::from("?:"),
            Self::Atomic => String::from("?>"),
            Self::PosLookahead => String::from("?="),
            Self::NegLookahead => String::from("?!"),
            Self::PosLookbehind => String::from("?<="),
            Self::NegLookbehind => String::from("?<!"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TernaryGroupId {
    Numbered(usize),
    Named(String),
}

impl TernaryGroupId {
    pub fn as_string(&self) -> String {
        match self {
            TernaryGroupId::Numbered(n) => n.to_string(),
            TernaryGroupId::Named(n) => n.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Group {
    NamedBackref {
        name: String,
    },
    Ternary {
        group_id: TernaryGroupId,
        yes_pat: Box<SubPattern>,
        no_pat: Option<Box<SubPattern>>,
    },
    Group {
        ext: Option<GroupExt>,
        flags: Flags,
        name: Option<String>,
        components: Vec<SubPattern>,
    },
}

impl Group {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let mut inner = pair.into_inner();
        inner.next(); // l_parens
        let fst = inner.next().ok_or(ReggieError::unexpected_eoi(char_ix))?;
        match fst.as_rule() {
            Rule::group_ext => Ok(Group::ext_group_from_pairs(fst, inner)?),
            Rule::sub_pattern => Ok(Group::plain_group_from_pairs(fst, inner)?),
            _ => Err(ReggieError::unexpected_input(fst).into()),
        }
    }
    pub(crate) fn plain_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Result<Self> {
        let mut c = vec![SubPattern::from_pair(fst)?];
        for p in inner.into_iter() {
            if p.as_rule() == Rule::sub_pattern {
                c.push(SubPattern::from_pair(p)?);
            }
        }
        Ok(Self::Group {
            ext: None,
            flags: Flags::empty(),
            name: None,
            components: c,
        })
    }
    pub(crate) fn ext_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Result<Self> {
        let (_, char_ix) = fst.line_col();
        let mut fst_inner = fst.into_inner();
        fst_inner.next(); // ?
        let ext_pair = fst_inner
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?;
        match ext_pair.as_rule() {
            Rule::noncapturing => Self::noncapturing_group_from_pairs(ext_pair, inner),
            Rule::atomic => Self::atomic_group_from_pairs(inner),
            Rule::pos_lookahead => Self::pos_lookahead_group_from_pairs(inner),
            Rule::neg_lookahead => Self::neg_lookahead_group_from_pairs(inner),
            Rule::pos_lookbehind => Self::pos_lookbehind_group_from_pairs(inner),
            Rule::neg_lookbehind => Self::neg_lookbehind_group_from_pairs(inner),
            Rule::named_backref => Self::named_backref_from_pairs(ext_pair),
            Rule::named => Self::named_group_from_pairs(ext_pair, inner),
            Rule::ternary => Self::ternary_group_from_pairs(ext_pair),
            _ => Err(ReggieError::unexpected_input(ext_pair).into()),
        }
    }
    pub fn name(&self) -> Option<String> {
        if let Group::Group { name, .. } = self {
            name.clone()
        } else {
            None
        }
    }
    pub(crate) fn group_from_subpatterns(
        components: Vec<SubPattern>,
        flags: Option<Flags>,
        name: Option<String>,
        ext: Option<GroupExt>,
    ) -> Self {
        Self::Group {
            flags: flags.unwrap_or(Flags::empty()),
            name,
            ext,
            components,
        }
    }
    pub(crate) fn is_indexed(&self) -> bool {
        matches!(self, Group::Group { ext: None, .. })
    }
    pub(crate) fn noncapturing_group_from_pairs(
        ext_pair: Pair<Rule>,
        inner: Pairs<'_, Rule>,
    ) -> Result<Self> {
        let flags = if let Some(matched_flags) = ext_pair.into_inner().next() {
            Flags::from_pair(matched_flags)?
        } else {
            Flags::empty()
        };
        let components = SubPattern::inner_components(inner)?;
        Ok(Self::Group {
            ext: Some(GroupExt::NonCapturing),
            name: None,
            components,
            flags,
        })
    }
    fn atomic_group_from_pairs(inner: Pairs<'_, Rule>) -> Result<Self> {
        Self::mk_ext_group(GroupExt::Atomic, inner)
    }
    fn pos_lookahead_group_from_pairs(inner: Pairs<'_, Rule>) -> Result<Self> {
        Self::mk_ext_group(GroupExt::PosLookahead, inner)
    }
    fn neg_lookahead_group_from_pairs(inner: Pairs<'_, Rule>) -> Result<Self> {
        Self::mk_ext_group(GroupExt::NegLookahead, inner)
    }
    fn pos_lookbehind_group_from_pairs(inner: Pairs<'_, Rule>) -> Result<Self> {
        Self::mk_ext_group(GroupExt::PosLookbehind, inner)
    }
    fn neg_lookbehind_group_from_pairs(inner: Pairs<'_, Rule>) -> Result<Self> {
        Self::mk_ext_group(GroupExt::NegLookbehind, inner)
    }
    fn named_backref_from_pairs(ext_pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = ext_pair.line_col();
        let name = ext_pair
            .into_inner()
            .skip(1) // ?
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .into_inner()
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .as_str()
            .into();
        Ok(Self::NamedBackref { name })
    }
    fn ternary_group_from_pairs(ext_pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = ext_pair.line_col();
        let mut inner = ext_pair.into_inner();
        let group = inner
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .into_inner()
            .skip(1) // (
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?;
        let group_id = match group.as_rule() {
            Rule::numbered_group_id => TernaryGroupId::Numbered(
                group
                    .as_str()
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(group))?,
            ),
            Rule::named_group_id => TernaryGroupId::Named(group.as_str().into()),
            _ => return Err(ReggieError::unexpected_input(group).into()),
        };
        if let Some(_) = inner.next() {
            if let Some(yp_inner) = inner.next() {
                let yes_pat = Box::new(SubPattern::from_pair(yp_inner)?);
                // skip |
                let no_pat = if inner.next().is_some() {
                    Some(Box::new(SubPattern::from_pair(
                        inner.next().ok_or(ReggieError::unexpected_eoi(char_ix))?,
                    )?))
                } else {
                    None
                };
                Ok(Self::Ternary {
                    group_id,
                    yes_pat,
                    no_pat,
                })
            } else {
                Err(ReggieError::unexpected_eoi(char_ix).into())
            }
        } else {
            Err(ReggieError::unexpected_eoi(char_ix).into())
        }
    }
    fn named_group_from_pairs(ext_pair: Pair<Rule>, inner: Pairs<'_, Rule>) -> Result<Self> {
        let (_, char_ix) = ext_pair.line_col();
        let mut ext_inner = ext_pair.into_inner();
        ext_inner.next(); // <
        let name: String = ext_inner
            .next()
            .ok_or(ReggieError::unexpected_eoi(char_ix))?
            .as_str()
            .into();
        let components = SubPattern::inner_components(inner)?;
        Ok(Self::Group {
            ext: None,
            flags: Flags::empty(),
            name: Some(name),
            components,
        })
    }
    fn mk_ext_group(ext: GroupExt, pairs: Pairs<'_, Rule>) -> Result<Self> {
        let components = SubPattern::inner_components(pairs)?;
        Ok(Self::Group {
            ext: Some(ext),
            name: None,
            components,
            flags: Flags::empty(),
        })
    }
    pub fn as_string(&self) -> String {
        match self {
            Group::NamedBackref { name } => format!("(?P={})", name),
            Group::Ternary {
                group_id,
                yes_pat,
                no_pat: None,
            } => format!("(?({}){})", group_id.as_string(), yes_pat.as_string()),
            Group::Ternary {
                group_id,
                yes_pat,
                no_pat: Some(no_pat),
            } => format!(
                "(?({}){}|{})",
                group_id.as_string(),
                yes_pat.as_string(),
                no_pat.as_string()
            ),
            Group::Group {
                ext: Some(ext),
                name: None,
                components: cs,
                ..
            } => {
                let mut s = format!("(?{}", ext.as_string());
                for component in cs.iter() {
                    write!(&mut s, "{}", component.as_string()).unwrap();
                }
                write!(&mut s, ")").unwrap();
                s
            }
            Group::Group {
                ext: None,
                name: Some(name),
                components: cs,
                ..
            } => {
                let mut s = format!("(?P<{}>", name);
                for component in cs.iter() {
                    write!(&mut s, "{}", component.as_string()).unwrap();
                }
                write!(&mut s, ")").unwrap();
                s
            }
            Group::Group {
                ext: None,
                name: None,
                ..
            } => unreachable!(),
            Group::Group {
                ext: Some(_),
                name: Some(_),
                ..
            } => unreachable!(),
        }
    }
    fn flags(&self) -> Flags {
        match self {
            Self::Group {
                components, flags, ..
            } => components
                .iter()
                .fold(flags.clone(), |acc, val| acc.combine(val.flags())),
            _ => Flags::empty(),
        }
    }
    pub fn indexed(&self) -> bool {
        matches!(self, Group::Group { ext: None, .. })
    }
    pub fn is_finite(&self) -> bool {
        //TODO(shr) similarly flawed
        match self {
            Group::NamedBackref { .. } => true,
            Group::Ternary {
                yes_pat, no_pat, ..
            } => yes_pat.is_finite() && no_pat.as_ref().map_or(true, |p| p.is_finite()),
            Group::Group { components, .. } => {
                for c in components.iter() {
                    if !c.is_finite() {
                        return false;
                    }
                }
                true
            }
        }
    }
    pub fn min_match_len(&self) -> usize {
        //TODO(shr) this isn't quite right
        match self {
            Group::NamedBackref { .. } => 0,
            Group::Ternary { yes_pat, .. } => yes_pat.min_match_len(),
            Group::Group {
                ext: Some(GroupExt::NonCapturing),
                ..
            } => 0,
            Group::Group { components, .. } => components.iter().map(|c| c.min_match_len()).sum(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_group_ext_as_string() {
        assert_eq!(String::from("?:"), GroupExt::NonCapturing.as_string());
        assert_eq!(String::from("?>"), GroupExt::Atomic.as_string());
        assert_eq!(String::from("?="), GroupExt::PosLookahead.as_string());
        assert_eq!(String::from("?!"), GroupExt::NegLookahead.as_string());
        assert_eq!(String::from("?<="), GroupExt::PosLookbehind.as_string());
        assert_eq!(String::from("?<!"), GroupExt::NegLookbehind.as_string());
    }
    #[test]
    fn test_group_as_string_named_backref() {
        assert_eq!(
            String::from("(?P=foo)"),
            Group::NamedBackref { name: "foo".into() }.as_string()
        );
    }
    // #[test]
    // fn test_group_as_string_ternary() {
    //     todo!()
    // }
    // #[test]
    // fn test_group_as_string_group() {
    //     todo!()
    // }
}
