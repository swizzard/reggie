use crate::{components::pattern::SubPattern, error::ReggieError, parser::Rule};
use anyhow::Result;
use pest::iterators::Pair;

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
    pub fn as_string(&self) -> String {
        todo!()
    }
    pub fn min_match_len(&self) -> usize {
        let mut min = usize::MAX;
        for sp in self.0.iter() {
            let mml = sp.min_match_len();
            if mml < min {
                min = mml
            }
        }
        min
    }
    pub fn is_finite(&self) -> bool {
        for sp in self.0.iter() {
            if !sp.is_finite() {
                return false;
            }
        }
        true
    }
}
