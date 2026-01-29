use crate::{error::ReggieError, parser::Rule};
use anyhow::Result;
use pest::iterators::{Pair, Pairs};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Q {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    NExact(usize),
    NTimes {
        min: Option<usize>,
        max: Option<usize>,
    },
}

impl Q {
    fn n_from_pair(inner: &mut Pairs<'_, Rule>, char_ix: usize) -> Result<Self> {
        let nt_match = inner.next().ok_or(ReggieError::unexpected_eoi(char_ix))?;
        let (_, nt_char_ix) = nt_match.line_col();
        let res = match nt_match.as_rule() {
            Rule::n_exact => Ok(Q::NExact(
                nt_match
                    .as_str()
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(nt_match))?,
            )),
            Rule::n_between => {
                let ent = nt_match.clone();
                let mut vals = nt_match.as_str().split(',');
                let min = vals
                    .next()
                    .ok_or(ReggieError::unexpected_eoi(nt_char_ix))?
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(nt_match))?;
                let max = vals
                    .next()
                    .ok_or(ReggieError::unexpected_eoi(nt_char_ix))?
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(ent))?;
                Ok(Q::NTimes {
                    min: Some(min),
                    max: Some(max),
                })
            }
            Rule::n_at_least => {
                let min = nt_match
                    .as_str()
                    .strip_suffix(',')
                    .ok_or(ReggieError::unexpected_eoi(nt_char_ix))?
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(nt_match))?;
                Ok(Q::NTimes {
                    min: Some(min),
                    max: None,
                })
            }
            Rule::n_at_most => {
                let max = nt_match
                    .as_str()
                    .strip_prefix(',')
                    .ok_or(ReggieError::unexpected_eoi(nt_char_ix))?
                    .parse::<usize>()
                    .map_err(|_| ReggieError::unexpected_input(nt_match))?;
                Ok(Q::NTimes {
                    min: None,
                    max: Some(max),
                })
            }
            _ => Err(ReggieError::unexpected_input(nt_match)),
        };
        Ok(res?)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum G {
    Greedy,
    NonGreedy,
    Possessive,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Quantifier {
    quantifier: Q,
    greed: G,
}

impl Quantifier {
    pub fn from_pair(pair: Pair<Rule>) -> Result<Self> {
        let (_, char_ix) = pair.line_col();
        let r = pair.as_rule();
        let ep = pair.clone();
        let mut pair_inner = pair.into_inner();
        if let Rule::quantifier = r {
            let mut quantifier = None;
            while let Some(q_match) = pair_inner.next() {
                let q_rule = q_match.as_rule();
                match q_rule {
                    Rule::question_mark => {
                        let _ = quantifier.insert(Quantifier::new(Q::ZeroOrOne));
                        break;
                    }
                    Rule::asterisk => {
                        let _ = quantifier.insert(Quantifier::new(Q::ZeroOrMore));
                        break;
                    }
                    Rule::plus => {
                        let _ = quantifier.insert(Quantifier::new(Q::OneOrMore));
                        break;
                    }
                    Rule::l_brace => {
                        let _ = quantifier
                            .insert(Quantifier::new(Q::n_from_pair(&mut pair_inner, char_ix)?));
                        break;
                    }
                    Rule::r_brace => break,
                    _ => return Err(ReggieError::unexpected_input(q_match).into()),
                }
            }
            let mut quantifier = quantifier.ok_or(ReggieError::unexpected_eoi(char_ix))?;
            while let Some(greed_match) = pair_inner.next() {
                match greed_match.as_rule() {
                    Rule::question_mark => quantifier.set_greed(G::NonGreedy),
                    Rule::plus => quantifier.set_greed(G::Possessive),
                    Rule::r_brace => continue,
                    _ => return Err(ReggieError::unexpected_input(greed_match).into()),
                }
            }
            Ok(quantifier)
        } else {
            Err(ReggieError::unexpected_input(ep).into())
        }
    }
    pub fn as_string(&self) -> String {
        let mut s = match self.quantifier {
            Q::ZeroOrOne => String::from("?"),
            Q::ZeroOrMore => String::from("*"),
            Q::OneOrMore => String::from("+"),
            Q::NExact(n) => format!("{{{}}}", n),
            Q::NTimes {
                min: Some(min),
                max: None,
            } => format!("{{{},}}", min),
            Q::NTimes {
                min: None,
                max: Some(max),
            } => format!("{{,{}}}", max),
            Q::NTimes {
                min: Some(min),
                max: Some(max),
            } => format!("{{{},{}}}", min, max),
            Q::NTimes {
                min: None,
                max: None,
            } => unreachable!(),
        };
        match self.greed {
            G::NonGreedy => {
                s.push_str("?");
            }
            G::Possessive => {
                s.push_str("+");
            }
            _ => (),
        };
        s
    }
    pub fn is_greedy(&self) -> bool {
        !matches!(self.greed, G::NonGreedy)
    }
    pub fn is_finite(&self) -> bool {
        matches!(
            self.quantifier,
            Q::ZeroOrOne | Q::NExact(_) | Q::NTimes { max: Some(_), .. }
        )
    }
    pub fn set_greed(&mut self, greed: G) {
        self.greed = greed;
    }
    pub fn set_quantifier(&mut self, quantifier: Q) {
        self.quantifier = quantifier;
    }
    pub(crate) fn min_len_multiplier(&self) -> usize {
        match self.quantifier {
            Q::ZeroOrOne | Q::ZeroOrMore => 0,
            Q::OneOrMore => 1,
            Q::NExact(n) => n,
            Q::NTimes { min, .. } => min.unwrap_or_default(),
        }
    }
    fn new(quantifier: Q) -> Self {
        Self {
            quantifier,
            greed: G::Greedy,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_quantifier_as_string() {
        assert_eq!(
            String::from("?"),
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("??"),
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::NonGreedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("?+"),
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::Possessive
            }
            .as_string()
        );
        assert_eq!(
            String::from("*"),
            Quantifier {
                quantifier: Q::ZeroOrMore,
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("+"),
            Quantifier {
                quantifier: Q::OneOrMore,
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("{1}"),
            Quantifier {
                quantifier: Q::NExact(1),
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("{2,}"),
            Quantifier {
                quantifier: Q::NTimes {
                    min: Some(2),
                    max: None
                },
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("{,2}"),
            Quantifier {
                quantifier: Q::NTimes {
                    min: None,
                    max: Some(2)
                },
                greed: G::Greedy
            }
            .as_string()
        );
        assert_eq!(
            String::from("{2,4}"),
            Quantifier {
                quantifier: Q::NTimes {
                    min: Some(2),
                    max: Some(4)
                },
                greed: G::Greedy
            }
            .as_string()
        );
    }
    #[test]
    fn test_quantifier_is_greedy() {
        assert!(
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::Greedy
            }
            .is_greedy()
        );
        assert!(
            !Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::NonGreedy
            }
            .is_greedy()
        );
        assert!(
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::Possessive
            }
            .is_greedy()
        );
    }
    #[test]
    fn test_quantifier_is_finite() {
        assert!(
            Quantifier {
                quantifier: Q::ZeroOrOne,
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            !Quantifier {
                quantifier: Q::ZeroOrMore,
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            !Quantifier {
                quantifier: Q::OneOrMore,
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            Quantifier {
                quantifier: Q::NExact(3),
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            !Quantifier {
                quantifier: Q::NTimes {
                    min: Some(3),
                    max: None
                },
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            Quantifier {
                quantifier: Q::NTimes {
                    min: None,
                    max: Some(3)
                },
                greed: G::Greedy
            }
            .is_finite()
        );
        assert!(
            Quantifier {
                quantifier: Q::NTimes {
                    min: Some(2),
                    max: Some(3)
                },
                greed: G::Greedy
            }
            .is_finite()
        );
    }
}
