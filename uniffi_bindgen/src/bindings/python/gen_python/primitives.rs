/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::backend::{CodeOracle, CodeType, Literal};
use crate::interface::Radix;
use askama::Template;
use paste::paste;
use std::fmt;

#[allow(unused_imports)]
use super::filters;

fn render_literal(_oracle: &dyn CodeOracle, literal: &Literal) -> String {
    match literal {
        Literal::Boolean(v) => {
            if *v {
                "True".into()
            } else {
                "False".into()
            }
        }
        Literal::String(s) => format!("\"{}\"", s),
        // https://docs.python.org/3/reference/lexical_analysis.html#integer-literals
        Literal::Int(i, radix, _) => match radix {
            Radix::Octal => format!("int(0o{:o})", i),
            Radix::Decimal => format!("{}", i),
            Radix::Hexadecimal => format!("{:#x}", i),
        },
        Literal::UInt(i, radix, _) => match radix {
            Radix::Octal => format!("0o{:o}", i),
            Radix::Decimal => format!("{}", i),
            Radix::Hexadecimal => format!("{:#x}", i),
        },
        Literal::Float(string, _type_) => string.clone(),

        _ => unreachable!("Literal"),
    }
}

macro_rules! impl_code_type_for_primitive {
    ($T:ty, $class_name:literal, $template_file:literal) => {
        paste! {
            #[derive(Template)]
            #[template(syntax = "py", escape = "none", path = $template_file)]
            pub struct $T;

            impl CodeType for $T  {
                fn type_label(&self, _oracle: &dyn CodeOracle) -> String {
                    $class_name.into()
                }

                fn literal(&self, oracle: &dyn CodeOracle, literal: &Literal) -> String {
                    render_literal(oracle, &literal)
                }

                fn lower(&self, oracle: &dyn CodeOracle, nm: &dyn fmt::Display) -> String {
                    format!("FfiConverter{}._lower({})", $class_name, oracle.var_name(nm))
                }

                fn write(&self, oracle: &dyn CodeOracle, nm: &dyn fmt::Display, target: &dyn fmt::Display) -> String {
                    format!("FfiConverter{}._write({}, {})", $class_name, oracle.var_name(nm), target)
                }

                fn lift(&self, _oracle: &dyn CodeOracle, nm: &dyn fmt::Display) -> String {
                    format!("FfiConverter{}._lift({})", $class_name, nm)
                }

                fn read(&self, _oracle: &dyn CodeOracle, nm: &dyn fmt::Display) -> String {
                    format!("FfiConverter{}._read({})", $class_name, nm)
                }

                fn helper_code(&self, _oracle: &dyn CodeOracle) -> Option<String> {
                    Some(self.render().unwrap())
                }
            }
        }
    }
}

impl_code_type_for_primitive!(BooleanCodeType, "Bool", "BooleanHelper.py");
impl_code_type_for_primitive!(StringCodeType, "String", "StringHelper.py");
impl_code_type_for_primitive!(Int8CodeType, "Int8", "Int8Helper.py");
impl_code_type_for_primitive!(Int16CodeType, "Int16", "Int16Helper.py");
impl_code_type_for_primitive!(Int32CodeType, "Int32", "Int32Helper.py");
impl_code_type_for_primitive!(Int64CodeType, "Int64", "Int64Helper.py");
impl_code_type_for_primitive!(UInt8CodeType, "UInt8", "UInt8Helper.py");
impl_code_type_for_primitive!(UInt16CodeType, "UInt16", "UInt16Helper.py");
impl_code_type_for_primitive!(UInt32CodeType, "UInt32", "UInt32Helper.py");
impl_code_type_for_primitive!(UInt64CodeType, "UInt64", "UInt64Helper.py");
impl_code_type_for_primitive!(Float32CodeType, "Float", "Float32Helper.py");
impl_code_type_for_primitive!(Float64CodeType, "Double", "Float64Helper.py");
