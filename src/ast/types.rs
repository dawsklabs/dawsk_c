use std::fmt::{ Display, Formatter, Result };
use crate::color::Color;

pub const MOUVE_COLOR: Color = Color::FgHex("#cba6f7");
pub const LAVENDAR_COLOR: Color = Color::FgHex("#b4befe");
pub const BLUE_COLOR: Color = Color::FgHex("#89b4fa");
pub const GREEN_COLOR: Color = Color::FgHex("#a6e3a1");
pub const FLAMINGO_COLOR: Color = Color::FgHex("#f2cdcd");
pub const PEACH_COLOR: Color = Color::FgHex("#fab387");
pub const MAROON_COLOR: Color = Color::FgHex("#eba0ac");
pub const RED_COLOR: Color = Color::FgHex("#f38ba8");
pub const SUBTEXT_COLOR: Color = Color::FgHex("#a6adc8");
pub const RESET_COLOR: Color = Color::Reset;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Primitive(Primitive),
    Tuple(Vec<TypeKind>),
    Vector(Box<TypeKind>),
    Set(Box<TypeKind>),
    Map(Box<TypeKind>, Box<TypeKind>),
    Custom(String), // e.g. structs, custom types, etc...
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            TypeKind::Primitive(p) => write!(f, "{}{}{}", MOUVE_COLOR, p, RESET_COLOR),
            TypeKind::Tuple(t) => {
                let types: Vec<String> = t.iter().map(|ty| format!("{}{}{}", BLUE_COLOR, ty.to_string(), RESET_COLOR)).collect();
                write!(f, "({})", types.join(", "))
            },
            TypeKind::Vector(v) => write!(f, "{}Vec{}<{}>", GREEN_COLOR, RESET_COLOR, v),
            TypeKind::Set(inner) => write!(f, "{}Set{}<{}>", GREEN_COLOR, RESET_COLOR, inner),
            TypeKind::Map(k, v) => write!(f, "{}Map{}<{}, {}>", GREEN_COLOR, RESET_COLOR, k, v),
            TypeKind::Custom(s) => write!(f, "{}", s),
            _ => write!(f, "<not yet implemented>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    Bool,
    Char,
    String,
}

impl Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            Primitive::I8 => write!(f, "i8"),
            Primitive::I16 => write!(f, "i16"),
            Primitive::I32 => write!(f, "i32"),
            Primitive::I64 => write!(f, "i64"),
            Primitive::U8 => write!(f, "u8"),
            Primitive::U16 => write!(f, "u16"),
            Primitive::U32 => write!(f, "u32"),
            Primitive::U64 => write!(f, "u64"),
            Primitive::F32 => write!(f, "f32"),
            Primitive::F64 => write!(f, "f64"),
            Primitive::Bool => write!(f, "bool"),
            Primitive::Char => write!(f, "char"),
            Primitive::String => write!(f, "str"),
            _ => write!(f, "<not yet implemented>"),
        }
    }
}