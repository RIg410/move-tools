use move_lang::errors::Errors;
use move_lang::expansion as libra_expansion;
use move_lang::naming as libra_naming;
use move_lang::parser as libra_parser;
use move_lang::shared::Address;
use move_lang::typing as libra_typing;

pub fn check_parsed_program(
    prog: libra_parser::ast::Program,
    sender_opt: Option<Address>,
) -> Result<(), Errors> {
    // expansion step
    let (e_program, errors) = libra_expansion::translate::program(prog, sender_opt);
    if !errors.is_empty() {
        return Err(errors);
    }
    // naming step
    let (n_program, errors) = libra_naming::translate::program(e_program, errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    // typechecking step
    let (_, errors) = libra_typing::translate::program(n_program, errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(())
}
