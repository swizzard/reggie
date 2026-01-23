#![allow(unused_imports)]
use pest::Parser;
use pest::iterators::Pair;
use reggie::*;

pub fn main() {
    let res = parser::PyRegexParser::parse(parser::Rule::component, r"[a-d]{5,9}?")
        .unwrap()
        .next()
        .unwrap();
    // println!("{:?}", res);
    let m = components::Component::from_pair(res);
    println!("{:?}", m);
    // let m = parser::PCRE2Parser::parse(parser::Rule::regex, r"a+bce[d-f]")
    //     .unwrap()
    //     .next()
    //     .unwrap()
    //     .into_inner();
    // for pair in m {
    //     let p = pair.into_inner().next().unwrap();
    //     // let c = components::MatchElement::from_pair(p);
    //     println!("{:?}", p);
    // }
    // .unwrap()
    // .next()
    // .unwrap();
    // let l = components::Literal::from_pair(m);
    // println!("{:?}", l);
}
