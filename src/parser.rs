// use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;
// use std::sync::LazyLock;

#[derive(Parser)]
#[grammar = "pyregex.pest"]
pub struct PyRegexParser;

// pub static PARSER: LazyLock<PrattParser<Rule>> =
//     LazyLock::new(|| PrattParser::new().op(Op::infix(Rule::pipe, Assoc::Left)));
