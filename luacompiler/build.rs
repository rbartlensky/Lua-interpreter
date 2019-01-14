extern crate cfgrammar;
extern crate lrlex;
extern crate lrpar;

use cfgrammar::yacc::{YaccKind, YaccOriginalActionKind};
use lrlex::LexerBuilder;
use lrpar::CTParserBuilder;

fn main() -> Result<(), Box<std::error::Error>> {
    let lex_rule_ids_map = CTParserBuilder::<u8>::new_with_storaget()
        .error_on_conflicts(false)
        .yacckind(YaccKind::Original(YaccOriginalActionKind::GenericParseTree))
        .process_file_in_src("lua5_3/lua5_3.y")?;
    LexerBuilder::<u8>::new()
        .rule_ids_map(lex_rule_ids_map)
        .process_file_in_src("lua5_3/lua5_3.l")?;
    Ok(())
}
