use std::fmt::{ Display, Formatter, Result };

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Primitive(Primitive),
    Tuple(Vec<TypeKind>),
    Vector(Box<TypeKind>),
    Custom(String),        // e.g. structs, custom types, etc...
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #![allow(unreachable_patterns)]
        match self {
            TypeKind::Primitive(p) => write!(f, "{}", p),
            TypeKind::Tuple(t) => {
                let types: Vec<String> = t.iter().map(|ty| ty.to_string()).collect();
                write!(f, "({})", types.join(", "))
            },
            TypeKind::Vector(v) => write!(f, "Vec<{}>", v),
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
            Primitive::String => write!(f, "string"),
            _ => write!(f, "<not yet implemented>"),
        }
    }
}