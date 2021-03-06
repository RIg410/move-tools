use std::str::FromStr;

use anyhow::{Result, Error, bail};

use libra::{
    compiler::{ModuleAccess_, ModuleIdent_, Type, Type_},
    vm::StructTag,
};
use libra::move_lang::parser::lexer::{Lexer, Tok};
use libra::move_lang::parser::syntax::{parse_num, parse_type};
use libra::account::Identifier;
use libra::prelude::*;

#[derive(Debug)]
pub struct TypeTagQuery {
    tt: TypeTag,

    /// Index of vector
    /// e.g.: `0x0::Mod::Res[i]`
    i: Option<u128>,
}

impl FromStr for TypeTagQuery {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        self::parse(s)
    }
}

impl Into<(TypeTag, Option<u128>)> for TypeTagQuery {
    fn into(self) -> (TypeTag, Option<u128>) {
        (self.tt, self.i)
    }
}

impl TypeTagQuery {
    pub fn into_inner(self) -> (TypeTag, Option<u128>) {
        self.into()
    }
}

fn unwrap_spanned_ty(ty: Type) -> Result<TypeTag, Error> {
    fn unwrap_spanned_ty_(ty: Type, this: Option<AccountAddress>) -> Result<TypeTag, Error> {
        let st = match ty.value {
            Type_::Apply(ma, mut ty_params) => {
                match (ma.value, this) {
                    // N
                    (ModuleAccess_::Name(name), this) => match name.value.as_ref() {
                        "bool" => TypeTag::Bool,
                        "u8" => TypeTag::U8,
                        "u64" => TypeTag::U64,
                        "u128" => TypeTag::U128,
                        "address" => TypeTag::Address,
                        "signer" => TypeTag::Signer,
                        "Vec" if ty_params.len() == 1 => TypeTag::Vector(
                            unwrap_spanned_ty_(ty_params.pop().unwrap(), this)
                                .unwrap()
                                .into(),
                        ),
                        _ => bail!(
                            "Could not parse input: type without struct name & module address"
                        ),
                    },
                    // M.S
                    (ModuleAccess_::ModuleAccess(_module, _struct_name), None) => {
                        bail!("Could not parse input: type without module address");
                    }
                    // M.S + parent address
                    (ModuleAccess_::ModuleAccess(name, struct_name), Some(this)) => {
                        TypeTag::Struct(StructTag {
                            address: this,
                            module: Identifier::new(name.0.value)?,
                            name: Identifier::new(struct_name.value)?,
                            type_params: ty_params
                                .into_iter()
                                .map(|ty| unwrap_spanned_ty_(ty, Some(this)))
                                .map(|res| match res {
                                    Ok(st) => st,
                                    Err(err) => panic!("{:?}", err),
                                })
                                .collect(),
                        })
                    }

                    // OxADDR.M.S
                    (ModuleAccess_::QualifiedModuleAccess(module_id, struct_name), _) => {
                        let ModuleIdent_ { name, address } = module_id.0.value;
                        let address = AccountAddress::new(address.to_u8());
                        TypeTag::Struct(StructTag {
                            address,
                            module: Identifier::new(name.0.value)?,
                            name: Identifier::new(struct_name.value)?,
                            type_params: ty_params
                                .into_iter()
                                .map(|ty| unwrap_spanned_ty_(ty, Some(address)))
                                .map(|res| match res {
                                    Ok(st) => st,
                                    Err(err) => panic!("{:?}", err),
                                })
                                .collect(),
                        })
                    }
                }
            }
            _ => {
                bail!("Could not parse input: unsupported type");
            }
        };

        Ok(st)
    }

    unwrap_spanned_ty_(ty, None)
}

pub fn parse(s: &str) -> Result<TypeTagQuery, Error> {
    let map_err = |err| anyhow!("{:?}", err);
    let mut lexer = Lexer::new(s, "query", Default::default());
    lexer.advance().map_err(map_err)?;

    let ty = parse_type(&mut lexer).map_err(map_err)?;
    let tt = unwrap_spanned_ty(ty)?;

    let mut i = None;
    while lexer.peek() != Tok::EOF {
        let tok = lexer.peek();
        lexer.advance().map_err(map_err)?;

        match tok {
            Tok::LBracket => i = parse_num(&mut lexer).ok(),
            _ => break,
        }
    }

    Ok(TypeTagQuery { tt, i })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert!(parse("0x1::Foo").is_err());

        let inputs = [
            "0x1::Foo::Res",
            "0x1::Foo::Res<Bar::Struct>",
            "0x1::Foo::Res<0x1::Bar::Struct>",
            "0x1::Foo::Res<0x1::Bar::T>[42]",
            "0x1::Foo::Res<0x1::Bar::T<u128>>[42]",
            "0x1::Foo::Res<Bar::T<u128>>",
            "0x1::Foo::Res<Vec<u128>>",
            "0x1::Foo::Res<Vec<u128>>[42]",
            "0x1::Foo::Bar::Ignored<Parts>",
        ];

        inputs
            .iter()
            .cloned()
            .map(|inp| (inp, parse(inp)))
            .for_each(|(inp, res)| {
                assert!(res.is_ok(), "failed on '{}'", inp);
                println!("{:?}", res.unwrap());
            });
    }
}
