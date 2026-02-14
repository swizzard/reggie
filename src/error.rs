use crate::parser::Rule;
use pest::iterators::Pair;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReggieError {
    #[error(transparent)]
    ParsingError(#[from] pest::error::Error<Rule>),
    #[error("Unexpected input {input} at character {char_ix:?}")]
    UnexpectedInput { input: String, char_ix: usize },
    #[error("Unexpected end of input at character {char_ix:?}")]
    UnexpectedEndOfInput { char_ix: usize },
    #[error("Invalid flag {bad_flag}")]
    InvalidFlag { bad_flag: char },
    #[error("Invalid literal {bad_literal}")]
    InvalidLiteral { bad_literal: String },
    #[error("Invalid ranges {bad_ranges:?}")]
    InvalidRanges { bad_ranges: Vec<(char, char)> },
    #[error("Invalid character class {bad_cclass}")]
    InvalidCharClass { bad_cclass: String },
    #[error("Pattern flags must be positive")]
    NegativePatternFlags,
}

impl ReggieError {
    pub(crate) fn unexpected_input(p: Pair<Rule>) -> Self {
        let (_, char_ix) = p.line_col();
        Self::UnexpectedInput {
            input: p.as_str().into(),
            char_ix,
        }
    }
    pub(crate) fn unexpected_eoi(char_ix: usize) -> Self {
        Self::UnexpectedEndOfInput { char_ix }
    }
}
