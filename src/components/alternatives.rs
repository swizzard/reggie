use crate::{
    components::{
        pattern::SubPattern,
        traits::{AsComponent, GroupLike},
    },
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::Pair;
use std::fmt::Write;

#[derive(Clone, Debug)]
pub struct Alternatives(Vec<SubPattern>);

impl Alternatives {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let mut inner = pair.into_inner();
        let mut alts: Vec<SubPattern> = Vec::new();
        while let Some(m) = inner.next() {
            match m.as_rule() {
                Rule::pipe => continue,
                Rule::sub_pattern => alts.push(SubPattern::from_pair(m)?),
                _ => alts.push(SubPattern::single_from_pair(m, &mut inner)?),
            }
        }
        Ok(Self(alts))
    }
}

impl AsComponent for Alternatives {
    fn as_string(&self) -> String {
        let cl = self.0.len();
        let mut cs = self.0.iter();
        let mut s = format!("{}|", cs.next().unwrap().as_string());
        let mut e = cs.enumerate();
        while let Some((ix, sp)) = e.next() {
            if ix + 1 >= cl {
                write!(s, "{}", sp.as_string());
            } else {
                write!(s, "{}|", sp.as_string());
            }
        }
        s
    }
    fn min_match_len(&self) -> usize {
        let mut min = usize::MAX;
        for sp in self.0.iter() {
            let mml = sp.min_match_len();
            if mml < min {
                min = mml
            }
        }
        min
    }
    fn is_finite(&self) -> bool {
        for sp in self.0.iter() {
            if !sp.is_finite() {
                return false;
            }
        }
        true
    }
}
impl GroupLike for Alternatives {
    fn sub_components(&self) -> Vec<impl AsComponent> {
        self.0.clone()
    }
}
