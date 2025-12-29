use std::fmt::{ Display, Formatter, Result };
use crate::color::{RESET_COLOR, GREEN_COLOR, BLUE_COLOR, MOUVE_COLOR, FLAMINGO_COLOR, SUBTEXT_COLOR};

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Primitive(Primitive),
    Tuple(Vec<TypeKind>),
    Generic {
        base: String,
        args: Vec<TypeKind>,
    },
    Custom(String), // e.g. structs, custom types, etc...
    Ref(Box<TypeKind>),
    MutRef(Box<TypeKind>),
    Untyped,
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            TypeKind::Primitive(p) => write!(f, "{}{}{}", MOUVE_COLOR, p, RESET_COLOR),
            TypeKind::Tuple(t) => {
                write!(f, "(")?;
                for (i, ty) in t.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            TypeKind::Custom(s) => write!(f, "{}{}{}", BLUE_COLOR, s, RESET_COLOR),
            TypeKind::Ref(inner) => write!(f, "{}&{}{}", FLAMINGO_COLOR, RESET_COLOR, inner),
            TypeKind::MutRef(inner) => write!(f, "{}&mut {}{}", FLAMINGO_COLOR, RESET_COLOR, inner),
            TypeKind::Generic { base, args } => {
                // println!("Generic type: {:?}", self);
                write!(f, "{}{}{}", GREEN_COLOR, base, RESET_COLOR)?;
                write!(f, "<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            TypeKind::Untyped => write!(f, "__UNTYPED"),
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
            Primitive::String => write!(f, "String"),
        }
    }
}