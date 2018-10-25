use lrpar::{Node, ParseError};
use std::io;

type ParseErr = (Option<Node<u8>>, Vec<ParseError<u8>>);

#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    LexError(lrpar::LexParseError<u8>),
    ParseError(ParseErr),
}

impl From<lrpar::LexParseError<u8>> for CliError {
    fn from(err: lrpar::LexParseError<u8>) -> CliError {
        CliError::LexError(err)
    }
}

impl From<ParseErr> for CliError {
    fn from(err: ParseErr) -> CliError {
        CliError::ParseError(err)
    }
}
