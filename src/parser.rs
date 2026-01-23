use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pcre2.pest"]
pub struct PCRE2Parser;
