//! This is a simple utility to replace one or more Wasm module imports with trapping stub functions.

use std::str::FromStr;

use anyhow::{anyhow, bail, Error, Result};
use clap::Parser;
use {
    std::{collections::HashMap, mem},
    wast::{
        core::{
            Expression, Func, FuncKind, Import, InlineExport, Instruction, ItemKind, ItemSig,
            ModuleField, ModuleKind, TypeUse,
        },
        parser::{self, ParseBuffer},
        token::Index,
        Wat,
    },
};

/// Replace one or more Wasm module imports with trapping stub functions.
///
/// The input module is read from stdin and the result is written to stdout.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Options {
    /// Stub all functions imported from the specified module name
    #[arg(short = 'm', long)]
    pub stub_module: Vec<String>,

    /// Stub the specified function import (syntax: "<module-name>:<function-name>")
    #[arg(short = 'f', long)]
    pub stub_function: Vec<Function>,
}

impl FromStr for Function {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (module, function) = s
            .split_once(':')
            .ok_or_else(|| anyhow!("expected <module-name>:<function-name>; got {s}"))?;

        Ok(Self {
            module: module.to_owned(),
            function: function.to_owned(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Function {
    pub module: String,
    pub function: String,
}

fn should_stub(options: &Options, module: &str, function: &str) -> bool {
    options.stub_module.iter().any(|m| module == m)
        || options
            .stub_function
            .iter()
            .any(|f| module == f.module && function == f.function)
}

fn make_translations(stubs: &[u32], imports: &[u32]) -> HashMap<u32, u32> {
    stubs
        .iter()
        .rev()
        .zip(imports.iter().rev())
        .filter_map(|(stub, import)| {
            if stub < import {
                Some([(*stub, *import), (*import, *stub)])
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

pub fn replace(options: Options, wasm: Vec<u8>) -> Result<Vec<u8>> {
    let wat = wasmprinter::print_bytes(&wasm)?;
    let buffer = ParseBuffer::new(&wat)?;
    let wat = parser::parse::<Wat>(&buffer)?;
    let mut module = match wat {
        Wat::Module(module) => module,
        Wat::Component(_) => bail!("components not yet supported"),
    };

    let fields = match &mut module.kind {
        ModuleKind::Text(fields) => fields,
        ModuleKind::Binary(_) => bail!("binary modules not yet supported"),
    };

    let stub = |span, ty, id, name| {
        ModuleField::Func(Func {
            span,
            id,
            name,
            exports: InlineExport { names: Vec::new() },
            kind: FuncKind::Inline {
                locals: Vec::new(),
                expression: Expression {
                    instrs: Box::new([Instruction::Unreachable]),
                },
            },
            ty,
        })
    };

    let mut stubs = Vec::new();
    let mut imports = Vec::new();
    let mut import_start = None;
    let mut translations = None;

    for (index, field) in fields.iter_mut().enumerate() {
        match field {
            ModuleField::Type(_) => (),
            ModuleField::Import(Import {
                span,
                module,
                field: import,
                item:
                    ItemSig {
                        id,
                        name,
                        kind: ItemKind::Func(ty),
                        ..
                    },
            }) => {
                import_start.get_or_insert(index);

                let count = (stubs.len() + imports.len()).try_into()?;

                if should_stub(&options, module, import) {
                    stubs.push(count);

                    *field = stub(
                        *span,
                        mem::replace(
                            ty,
                            TypeUse {
                                index: None,
                                inline: None,
                            },
                        ),
                        id.take(),
                        name.take(),
                    );
                } else {
                    imports.push(count);
                }
            }
            ModuleField::Func(func) => {
                let translations =
                    translations.get_or_insert_with(|| make_translations(&stubs, &imports));

                if let FuncKind::Inline {
                    expression: Expression { instrs },
                    ..
                } = &mut func.kind
                {
                    for instr in instrs.iter_mut() {
                        if let Instruction::Call(Index::Num(value, _)) = instr {
                            if let Some(&new_value) = translations.get(value) {
                                *value = new_value;
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    if let Some(start) = import_start {
        let translations = translations.get_or_insert_with(|| make_translations(&stubs, &imports));

        for &import in &imports {
            if let Some(&stub) = translations.get(&import) {
                fields.swap(
                    start + usize::try_from(import)?,
                    start + usize::try_from(stub)?,
                );
            }
        }
    }

    Ok(module.encode()?)
}
