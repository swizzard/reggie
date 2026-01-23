use crate::parser::Rule;
use disjoint_ranges::{DisjointRange, UnaryRange};
use pest::iterators::{Pair, Pairs};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct Pattern {
    flags: Flags,
    components: Vec<Component>,
}

impl Pattern {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let mut flags = Flags::new();
        let mut components = Vec::new();
        while let Some(matched) = inner.next() {
            match matched.as_rule() {
                Rule::component => components.push(Component::from_pair(matched)),
                Rule::whole_pattern_flag => {
                    let mut parsed_flags = Flags::from_whole_pattern_pair(matched);
                    std::mem::swap(&mut flags, &mut parsed_flags);
                }
                other => {
                    println!("actually {:?}", other);
                    unreachable!()
                }
            }
        }
        Self { flags, components }
    }
}

#[derive(Clone, Debug)]
pub struct Component {
    el: Element,
    quantifier: Option<Quantifier>,
}

impl Component {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let el = Element::from_pair(inner.next().unwrap());
        let quantifier = inner.next().map(|p| Quantifier::from_pair(p));
        Self { el, quantifier }
    }
    pub fn as_string(&self) -> String {
        if let Some(q) = self.quantifier {
            format!("{}{}", self.el.as_string(), q.as_string())
        } else {
            self.el.as_string()
        }
    }
}

#[derive(Clone, Debug)]
pub enum Element {
    CharSet(CharSet),
    Literal(Literal),
}

impl Element {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::char_set => Self::CharSet(CharSet::from_pair(pair)),
            Rule::literals => Self::Literal(Literal::from_pair(pair)),
            _ => {
                println!("actually {:?}", pair);
                unreachable!()
            }
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::CharSet(cs) => cs.as_string(),
            Self::Literal(l) => l.as_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Literal(String);

impl Literal {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let r = pair.as_rule();
        if let Rule::literals = r {
            Self(String::from(pair.as_str()))
        } else {
            println!("actually {:?}", r);
            unreachable!()
        }
    }
    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Clone, Debug)]
pub struct CharSet {
    char_ranges: DisjointRange<char>,
}

impl CharSet {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let r = pair.as_rule();
        if let Rule::char_set = r {
            let mut char_ranges = DisjointRange::empty();
            let mut negated = false;
            let mut pairs_iter = pair.into_inner();
            while let Some(p) = pairs_iter.next() {
                match p.as_rule() {
                    Rule::set_negation => negated = true,
                    Rule::char_range => {
                        let mut inner = p.into_inner();
                        let low = inner.next().unwrap().as_str().chars().nth(0).unwrap();
                        inner.next();
                        let high = inner.next().unwrap().as_str().chars().nth(0).unwrap();
                        char_ranges.add_unary_range(UnaryRange::new_unchecked(low, high));
                    }
                    Rule::hyphen => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('-', '-'))
                    }
                    Rule::set_literal => {
                        let c = p.as_str().chars().nth(0).unwrap();
                        char_ranges.add_unary_range(UnaryRange::new_unchecked(c, c));
                    }
                    Rule::escaped_hyphen => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('-', '-'));
                    }
                    Rule::caret => {
                        char_ranges.add_unary_range(UnaryRange::new_unchecked('^', '^'));
                    }
                    Rule::char_class => {
                        let cls = CharClass::from_pair(p);
                        char_ranges.add_disjoint_range(cls.to_range());
                    }
                    Rule::l_sq | Rule::r_sq => continue,
                    _ => {
                        println!("got {:}", p);
                        unreachable!()
                    }
                };
            }
            if negated {
                Self {
                    char_ranges: char_ranges.complement(),
                }
            } else {
                Self { char_ranges }
            }
        } else {
            println!("actually {:?}", r);
            unreachable!()
        }
    }
    pub fn as_string(&self) -> String {
        let mut s = String::from("[");
        for subrange in self.char_ranges.ranges_iter() {
            let (low, high) = subrange.as_bounds();
            s.push_str(format!("{}-{}", low, high).as_str());
        }
        s.push_str("]");
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CClass {
    D,
    S,
    W,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharClass {
    class: CClass,
    negated: bool,
}

impl CharClass {
    pub fn to_range(&self) -> DisjointRange<char> {
        let range = match self.class {
            CClass::D => CharClass::digit_range(),
            CClass::S => CharClass::whitespace_range(),
            CClass::W => CharClass::word_range(),
        };
        if self.negated {
            range.complement()
        } else {
            range
        }
    }
    fn digit_range() -> DisjointRange<char> {
        DisjointRange::new_single_range_unchecked('0', '9')
    }
    fn whitespace_range() -> DisjointRange<char> {
        DisjointRange::from_bounds_unchecked([('\t', '\r'), (' ', ' ')])
    }
    fn word_range() -> DisjointRange<char> {
        DisjointRange::from_bounds_unchecked([('a', 'z'), ('A', 'Z'), ('0', '9')])
    }
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        inner.next(); // backslash
        let c = inner.next().unwrap().as_str();
        match c {
            "d" => Self {
                class: CClass::D,
                negated: false,
            },
            "D" => Self {
                class: CClass::D,
                negated: true,
            },
            "s" => Self {
                class: CClass::S,
                negated: false,
            },
            "S" => Self {
                class: CClass::S,
                negated: true,
            },
            "w" => Self {
                class: CClass::W,
                negated: false,
            },
            "W" => Self {
                class: CClass::W,
                negated: true,
            },
            _ => {
                println!("c {:?}", c);
                unreachable!()
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Q {
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
    fn n_from_pair(inner: &mut Pairs<'_, Rule>) -> Self {
        let nt_match = inner.next().unwrap();
        let res = match nt_match.as_rule() {
            Rule::n_exact => Q::NExact(nt_match.as_str().parse::<usize>().unwrap()),
            Rule::n_between => {
                let mut vals = nt_match.as_str().split(',');
                let min = vals.next().unwrap().parse::<usize>().unwrap();
                let max = vals.next().unwrap().parse::<usize>().unwrap();
                Q::NTimes {
                    min: Some(min),
                    max: Some(max),
                }
            }
            Rule::n_at_least => {
                let min = nt_match
                    .as_str()
                    .strip_suffix(',')
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                Q::NTimes {
                    min: Some(min),
                    max: None,
                }
            }
            Rule::n_at_most => {
                let max = nt_match
                    .as_str()
                    .strip_prefix(',')
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                Q::NTimes {
                    min: None,
                    max: Some(max),
                }
            }
            other => {
                println!("actually {:?}", other);
                unreachable!()
            }
        };
        res
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum G {
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
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let r = pair.as_rule();
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
                        let _ = quantifier.insert(Quantifier::new(Q::n_from_pair(&mut pair_inner)));
                        break;
                    }
                    Rule::r_brace => break,
                    other => {
                        println!("q_rule actually {:?}", other);
                        unreachable!()
                    }
                }
            }
            let mut quantifier = quantifier.unwrap();
            while let Some(greed_match) = pair_inner.next() {
                match greed_match.as_rule() {
                    Rule::question_mark => quantifier.set_greed(G::NonGreedy),
                    Rule::plus => quantifier.set_greed(G::Possessive),
                    Rule::r_brace => continue,
                    other => {
                        println!("greed_match actually {:?}", other);
                        unreachable!()
                    }
                }
            }
            quantifier
        } else {
            println!("quantifier actually {:?}", r);
            unreachable!()
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

    fn new(quantifier: Q) -> Self {
        Self {
            quantifier,
            greed: G::Greedy,
        }
    }
    fn set_greed(&mut self, greed: G) {
        self.greed = greed;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Flag {
    Ascii,
    Ignorecase,
    Locale,
    Multiline,
    Dotall,
    Unicode,
    Verbose,
}

impl Flag {
    fn as_str(&self) -> &'static str {
        match self {
            Flag::Ascii => "a",
            Flag::Ignorecase => "i",
            Flag::Locale => "L",
            Flag::Multiline => "m",
            Flag::Dotall => "s",
            Flag::Unicode => "u",
            Flag::Verbose => "x",
        }
    }
    pub fn from_char(c: char) -> Self {
        match c {
            'a' => Self::Ascii,
            'i' => Self::Ignorecase,
            'L' => Self::Locale,
            'm' => Self::Multiline,
            's' => Self::Dotall,
            'u' => Self::Unicode,
            'x' => Self::Verbose,
            _ => unreachable!(),
        }
    }
    pub fn as_string(&self) -> String {
        String::from(self.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flags(HashSet<Flag>);

impl Flags {
    pub fn new() -> Self {
        Self(HashSet::new())
    }
    pub fn add(&mut self, flag: Flag) {
        self.0.insert(flag);
    }
    pub fn remove(&mut self, flag: &Flag) {
        self.0.remove(flag);
    }
    pub fn as_string(&self) -> String {
        let mut s = String::from("?");
        for flag in self.0.iter() {
            s.push_str(flag.as_str())
        }
        s
    }
    pub fn from_whole_pattern_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        inner.next(); // (?
        let flag_match = inner.next().unwrap();
        if flag_match.as_rule() == Rule::flags {
            let mut flags = Flags::new();
            for c in flag_match.as_str().chars() {
                flags.add(Flag::from_char(c));
            }
            flags
        } else {
            println!("actually {:?}", flag_match.as_rule());
            unreachable!()
        }
    }
}
