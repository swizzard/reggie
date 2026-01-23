use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pyregex.pest"]
pub struct PyRegexParser;
