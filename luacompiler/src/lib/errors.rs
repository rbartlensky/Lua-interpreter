use lrpar::{LexParseError, Node, ParseError};
use std::io;

type ParseErr = (Option<Node<u8>>, Vec<ParseError<u8>>);

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    LexError(Vec<LexParseError<u8>>),
    ParseError(ParseErr),
}

impl From<Vec<LexParseError<u8>>> for CliError {
    fn from(err: Vec<LexParseError<u8>>) -> CliError {
        CliError::LexError(err)
    }
}

impl From<ParseErr> for CliError {
    fn from(err: ParseErr) -> CliError {
        CliError::ParseError(err)
    }
}
