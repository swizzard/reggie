use crate::{
    components::{
        CClass, CharSet, Element, Flags, Group, GroupExt, Quantifier, pattern::SubPattern,
    },
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
    pub fn as_string(&self) -> String {
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
    pub(crate) quantifiable: Quantifiable,
    pub(crate) quantifier: Option<Quantifier>,
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
    pub(crate) fn subpatterns_to_group(
        components: Vec<SubPattern>,
        flags: Option<Flags>,
        name: Option<String>,
        ext: Option<GroupExt>,
    ) -> Self {
        Self {
            quantifier: None,
            quantifiable: Quantifiable::Group(Group::group_from_subpatterns(
                components, flags, name, ext,
            )),
        }
    }
    pub(crate) fn new_char_set_from_ranges(
        ranges: Vec<(char, char)>,
        quantifier: Option<Quantifier>,
    ) -> Result<Self> {
        Ok(Self {
            quantifier,
            quantifiable: Quantifiable::Element(Element::CharSet(CharSet::from_ranges(ranges)?)),
        })
    }
    pub(crate) fn new_char_class(cc: CClass, quantifier: Option<Quantifier>) -> Self {
        Self {
            quantifier,
            quantifiable: Quantifiable::Element(Element::CharSet(CharSet::from_cclass(cc))),
        }
    }
    pub(crate) fn new_literal(lit: String, quantifier: Option<Quantifier>) -> Self {
        Self {
            quantifier,
            quantifiable: Quantifiable::Element(Element::new_literal(lit)),
        }
    }
    pub fn as_string(&self) -> String {
        if let Some(q) = self.quantifier {
            format!("{}{}", self.quantifiable.as_string(), q.as_string())
        } else {
            self.quantifiable.as_string()
        }
    }
    pub fn flags(&self) -> Flags {
        Flags::empty()
    }
    pub fn indexed(&self) -> bool {
        false
    }
    pub fn min_match_len(&self) -> usize {
        self.quantifiable.min_match_len()
            * self.quantifier.map(|q| q.min_len_multiplier()).unwrap_or(1)
    }
    pub fn is_finite(&self) -> bool {
        self.quantifier.map(|q| q.is_finite()).unwrap_or(true)
    }
}
