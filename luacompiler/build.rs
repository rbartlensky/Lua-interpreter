extern crate lrlex;
extern crate lrpar;

use lrlex::LexerBuilder;
use lrpar::ActionKind;
use lrpar::CTParserBuilder;

fn main() -> Result<(), Box<std::error::Error>> {
    let mut ct = CTParserBuilder::<u8>::new_with_storaget()
        .error_on_conflicts(false)
        .action_kind(ActionKind::GenericParseTree);
    let lex_rule_ids_map = ct.process_file_in_src("lua5_3/lua5_3.y")?;
    LexerBuilder::new()
        .rule_ids_map(lex_rule_ids_map)
        .process_file_in_src("lua5_3/lua5_3.l")?;
    Ok(())
}
