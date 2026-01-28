use crate::parser::Rule;
use disjoint_ranges::{DisjointRange, UnaryRange};
use pest::iterators::{Pair, Pairs};
use std::{collections::HashSet, fmt::Write};

#[derive(Clone, Debug)]
pub struct Pattern {
    flags: Flags,
    sub_patterns: Vec<SubPattern>,
}

impl Pattern {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        let mut flags = Flags::new();
        let mut sub_patterns = Vec::new();
        while let Some(matched) = inner.next() {
            match matched.as_rule() {
                Rule::sub_pattern => sub_patterns.push(SubPattern::from_pair(matched)),
                Rule::whole_pattern_flags => {
                    let mut parsed_flags = Flags::from_whole_pattern_pair(matched);
                    std::mem::swap(&mut flags, &mut parsed_flags);
                }
                other => {
                    println!("actually {:?}", other);
                    unreachable!()
                }
            }
        }
        Self {
            flags,
            sub_patterns,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SubPattern {
    Quantifiable {
        el: Element,
        quantifier: Option<Quantifier>,
    },
    ZeroWidthLiteral(ZeroWidthLiteral),
    Comment(String),
    Group(Group),
}

impl SubPattern {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();
        if let Some(p) = inner.next() {
            match p.as_rule() {
                Rule::literals | Rule::char_set => SubPattern::quantifiable_from_pair(p, inner),
                Rule::zero_width_literal => SubPattern::zwl_from_pair(p),
                Rule::comment_group => SubPattern::comment_group_from_pair(p),
                Rule::group => SubPattern::group_from_pair(p),
                other => {
                    println!("component from_pair actually {:?}", other);
                    unreachable!()
                }
            }
        } else {
            println!("from_pair {:?} inner {:?}", rule, inner);
            unreachable!()
        }
    }
    fn inner_components(inner: Pairs<'_, Rule>) -> Vec<Self> {
        inner
            .map(|p| match p.as_rule() {
                Rule::sub_pattern => Some(Self::from_pair(p)),
                Rule::r_parens => None,
                other => {
                    println!("inner_components actually {:?}", other);
                    unreachable!()
                }
            })
            .flatten()
            .collect()
    }

    fn quantifiable_from_pair(pair: Pair<Rule>, mut inner: Pairs<'_, Rule>) -> Self {
        let el = Element::from_pair(pair);
        let quantifier = inner.next().map(|p| Quantifier::from_pair(p));
        Self::Quantifiable { el, quantifier }
    }
    fn zwl_from_pair(pair: Pair<Rule>) -> Self {
        Self::ZeroWidthLiteral(ZeroWidthLiteral::from_pair(pair))
    }
    fn comment_group_from_pair(pair: Pair<Rule>) -> Self {
        let inner = pair.into_inner();
        let content = inner.skip(3).next().unwrap(); // (?#
        Self::Comment(content.as_str().into())
    }
    fn group_from_pair(pair: Pair<Rule>) -> Self {
        let mut inner = pair.into_inner();
        inner.next(); // l_parens
        let fst = inner.next().unwrap();
        match fst.as_rule() {
            Rule::group_ext => SubPattern::ext_group_from_pairs(fst, inner),
            Rule::sub_pattern => SubPattern::plain_group_from_pairs(fst, inner),
            _ => unreachable!(),
        }
    }
    fn ext_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        Self::Group(Group::ext_group_from_pairs(fst, inner))
    }
    fn plain_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        Self::Group(Group::plain_group_from_pairs(fst, inner))
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::Quantifiable {
                el,
                quantifier: Some(q),
            } => format!("{}{}", el.as_string(), q.as_string()),
            Self::Quantifiable {
                el,
                quantifier: None,
            } => el.as_string(),
            Self::ZeroWidthLiteral(zwl) => zwl.as_string(),
            Self::Comment(c) => format!("(?#{}", c),
            Self::Group(g) => g.as_string(),
        }
    }
    pub fn min_match_len(&self) -> usize {
        match self {
            Self::Quantifiable { el, quantifier } => {
                el.min_match_len() * quantifier.map(|q| q.min_len_multiplier()).unwrap_or(1)
            }
            _ => todo!(),
        }
    }
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Quantifiable { quantifier, .. } => {
                quantifier.map(|q| q.is_finite()).unwrap_or(true)
            }
            _ => todo!(),
        }
    }
    pub fn flags(&self) -> Option<GroupFlags> {
        match self {
            SubPattern::Group(g) => g.flags(),
            _ => None,
        }
    }
}

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
    fn as_string(&self) -> String {
        match self {
            Self::NonCapturing => String::from("?:"),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TernaryGroupId {
    Numbered(usize),
    Named(String),
}

impl TernaryGroupId {
    fn as_string(&self) -> String {
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
        flags: GroupFlags,
        name: Option<String>,
        components: Vec<SubPattern>,
    },
}

impl Group {
    fn plain_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        let mut c = vec![SubPattern::from_pair(fst)];
        for p in inner.into_iter() {
            if p.as_rule() == Rule::sub_pattern {
                c.push(SubPattern::from_pair(p));
            }
        }
        Self::Group {
            ext: None,
            flags: GroupFlags::empty(),
            name: None,
            components: c,
        }
    }
    fn ext_group_from_pairs(fst: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        let mut fst_inner = fst.into_inner();
        fst_inner.next(); // ?
        let ext_pair = fst_inner.next().unwrap();
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
            // Rule::l_parens => Self::immediate_group_from_pairs(fst_inner),
            other => {
                println!("ext_group_from_pairs actually {:?}", other);
                unreachable!()
            }
        }
    }
    fn as_string(&self) -> String {
        match self {
            Group::NamedBackref { name } => format!("(?P={}", name),
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
    fn flags(&self) -> Option<GroupFlags> {
        match self {
            Self::Group {
                components, flags, ..
            } => {
                if flags.is_empty() {
                    for comp in components.iter() {
                        let f = comp.flags();
                        if f.is_some() {
                            return f;
                        }
                    }
                    None
                } else {
                    Some(flags.clone())
                }
            }
            _ => None,
        }
    }
    fn noncapturing_group_from_pairs(ext_pair: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        let flags = if let Some(matched_flags) = ext_pair.into_inner().next() {
            GroupFlags::from_pair(matched_flags)
        } else {
            GroupFlags::empty()
        };
        let components = SubPattern::inner_components(inner);
        Self::Group {
            ext: Some(GroupExt::NonCapturing),
            name: None,
            components,
            flags,
        }
    }
    fn atomic_group_from_pairs(inner: Pairs<'_, Rule>) -> Self {
        Self::mk_ext_group(GroupExt::Atomic, inner)
    }
    fn pos_lookahead_group_from_pairs(inner: Pairs<'_, Rule>) -> Self {
        Self::mk_ext_group(GroupExt::PosLookahead, inner)
    }
    fn neg_lookahead_group_from_pairs(inner: Pairs<'_, Rule>) -> Self {
        Self::mk_ext_group(GroupExt::NegLookahead, inner)
    }
    fn pos_lookbehind_group_from_pairs(inner: Pairs<'_, Rule>) -> Self {
        Self::mk_ext_group(GroupExt::PosLookbehind, inner)
    }
    fn neg_lookbehind_group_from_pairs(inner: Pairs<'_, Rule>) -> Self {
        Self::mk_ext_group(GroupExt::NegLookbehind, inner)
    }
    fn named_backref_from_pairs(ext_pair: Pair<Rule>) -> Self {
        let name = ext_pair
            .into_inner()
            .skip(1) // ?
            .next()
            .unwrap()
            .into_inner()
            .next()
            .unwrap()
            .as_str()
            .into();
        Self::NamedBackref { name }
    }
    fn ternary_group_from_pairs(ext_pair: Pair<Rule>) -> Self {
        let mut inner = ext_pair.into_inner();
        let group = inner
            .next()
            .unwrap()
            .into_inner()
            .skip(1) // (
            .next()
            .unwrap();
        let group_id = match group.as_rule() {
            Rule::numbered_group_id => {
                TernaryGroupId::Numbered(group.as_str().parse::<usize>().unwrap())
            }
            Rule::named_group_id => TernaryGroupId::Named(group.as_str().into()),
            _ => unreachable!(),
        };
        let yes_pat = Box::new(SubPattern::from_pair(inner.next().unwrap()));
        // skip |
        let no_pat = if inner.next().is_some() {
            Some(Box::new(SubPattern::from_pair(inner.next().unwrap())))
        } else {
            None
        };
        Self::Ternary {
            group_id,
            yes_pat,
            no_pat,
        }
    }
    fn named_group_from_pairs(ext_pair: Pair<Rule>, inner: Pairs<'_, Rule>) -> Self {
        let mut ext_inner = ext_pair.into_inner();
        ext_inner.next(); // <
        let name: String = ext_inner.next().unwrap().as_str().into();
        let components = SubPattern::inner_components(inner);
        Self::Group {
            ext: None,
            flags: GroupFlags::empty(),
            name: Some(name),
            components,
        }
    }
    fn mk_ext_group(ext: GroupExt, pairs: Pairs<'_, Rule>) -> Self {
        let components = SubPattern::inner_components(pairs);
        Self::Group {
            ext: Some(ext),
            name: None,
            components,
            flags: GroupFlags::empty(),
        }
    }
    // fn immediate_group_from_pairs(pairs: Pairs<'_, Rule>) -> Self {
    //     let components = Component::inner_components(pairs);
    //     Self::Group {
    //         ext: None,
    //         name: None,
    //         components,
    //         flags: GroupFlags::empty(),
    //     }
    // }
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
    pub fn min_match_len(&self) -> usize {
        match self {
            Self::CharSet(cs) => cs.min_match_len(),
            Self::Literal(l) => l.min_match_len(),
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
    pub fn min_match_len(&self) -> usize {
        self.0.len()
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
    pub fn min_match_len(&self) -> usize {
        1
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
    pub fn is_greedy(&self) -> bool {
        !matches!(self.greed, G::NonGreedy)
    }
    pub fn is_finite(&self) -> bool {
        match self.quantifier {
            Q::ZeroOrMore | Q::OneOrMore => false,
            _ => true,
        }
    }
    pub fn set_greed(&mut self, greed: G) {
        self.greed = greed;
    }
    pub fn set_quantifier(&mut self, quantifier: Q) {
        self.quantifier = quantifier;
    }
    fn min_len_multiplier(&self) -> usize {
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
    fn new() -> Self {
        Self(HashSet::new())
    }
    fn add(&mut self, flag: Flag) {
        self.0.insert(flag);
    }
    fn remove(&mut self, flag: &Flag) {
        self.0.remove(flag);
    }
    pub fn as_string(&self) -> String {
        let mut s = String::from("?");
        for flag in self.0.iter() {
            s.push_str(flag.as_str())
        }
        s
    }
    fn from_whole_pattern_pair(pair: Pair<Rule>) -> Self {
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
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupFlags {
    pos: Flags,
    neg: Flags,
}

impl GroupFlags {
    fn empty() -> Self {
        Self {
            pos: Flags::new(),
            neg: Flags::new(),
        }
    }
    fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.neg.is_empty()
    }

    fn from_pair(pair: Pair<Rule>) -> Self {
        let mut pos = Flags::new();
        let mut neg = Flags::new();
        let mut s = pair.as_str().split('-');
        for c in s.next().unwrap().chars() {
            pos.add(Flag::from_char(c));
        }
        if let Some(neg_flag_str) = s.next() {
            for c in neg_flag_str.chars() {
                neg.add(Flag::from_char(c))
            }
        };
        Self { pos, neg }
    }
    fn as_string(&self) -> String {
        if !self.neg.0.is_empty() {
            format!("{}-{}", self.pos.as_string(), self.neg.as_string())
        } else {
            self.pos.as_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ZeroWidthLiteral {
    InputStart,
    InputEnd,
    WordBoundary,
    NotWordBoundary,
}

impl ZeroWidthLiteral {
    pub fn from_pair(pair: Pair<Rule>) -> Self {
        match pair.as_str() {
            "\\A" | "\\a" => Self::InputStart,
            "\\b" => Self::WordBoundary,
            "\\B" => Self::NotWordBoundary,
            "\\Z" | "\\z" => Self::InputEnd,
            _ => unreachable!(),
        }
    }
    pub fn as_string(&self) -> String {
        match self {
            Self::InputStart => String::from("\\a"),
            Self::InputEnd => String::from("\\z"),
            Self::NotWordBoundary => String::from("\\B"),
            Self::WordBoundary => String::from("\\b"),
        }
    }
}
