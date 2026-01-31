use crate::{
    components::{element::Element, groups::Group, quantifiers::Quantifier},
    error::ReggieError,
    parser::Rule,
};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};

#[derive(Clone, Debug)]
pub enum Quantifiable {
    Element(Element),
    Group(Group),
}

impl Quantifiable {
    fn as_string(&self) -> String {
        match self {
            Quantifiable::Element(e) => e.as_string(),
            Quantifiable::Group(g) => g.as_string(),
        }
    }
    fn min_match_len(&self) -> usize {
        match self {
            Self::Element(e) => e.min_match_len(),
            Self::Group(g) => g.min_match_len(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Quantified {
    quantifiable: Quantifiable,
    quantifier: Option<Quantifier>,
}

impl Quantified {
    pub fn from_pair(pair: Pair<Rule>, inner: &mut Pairs<'_, Rule>) -> Result<Self> {
        let quantifiable = match pair.as_rule() {
            Rule::char_set => Quantifiable::Element(Element::charset_from_pair(pair)?),
            Rule::literals => Quantifiable::Element(Element::literals_from_pair(pair)?),
            Rule::group => Quantifiable::Group(Group::from_pair(pair)?),
            other => {
                println!("quantified from_pair actually {:?}", other);
                return Err(ReggieError::unexpected_input(pair).into());
            }
        };
        let p = inner.peek();
        let quantifier = if let Some(p) = p
            && p.as_rule() == Rule::quantifier
        {
            let p = inner.next().unwrap();
            Quantifier::from_pair(p)?
        } else {
            None
        };
        Ok(Quantified {
            quantifiable,
            quantifier,
        })
    }
    pub fn as_string(&self) -> String {
        if let Some(q) = self.quantifier {
            format!("{}{}", self.quantifiable.as_string(), q.as_string())
        } else {
            self.quantifiable.as_string()
        }
    }
    pub fn min_match_len(&self) -> usize {
        self.quantifiable.min_match_len()
            * self.quantifier.map(|q| q.min_len_multiplier()).unwrap_or(1)
    }
    pub fn is_finite(&self) -> bool {
        self.quantifier.map(|q| q.is_finite()).unwrap_or(true)
    }
}
