# Syntax



dec(const m: u8 = b'e'; // constants
dec m: i8 = -15; // variables

dec mut x: f32 = 18e-1;

dec t: f32 = 4 - 0.3;

dec mut z: f32 = x + t; // first 'pub', 'mut' afterwards

dec c = &&mut z;  // c: &&mut f32 (explizit); ref takes the same type (should be logical) but as ref...

dec mut y: = **c; // y should then have the value of z

pub struct MyStruct {
    pub a: i8,
    pub b: u16,
    pub c: String,
    x: f32,
    pub t: DynArr\<u8>,
}

impl Add for MyStruct {
    fn add(self, ...)
}

pub trait Printable {
    fn print(&self);
}

impl Printable for MyStruct {
    fn print(&self) {
        println!("MyStruct values");
    }
}

pub enum Type {
    A(i32),
    C,
}

pub enum MyEnum {
    A(Type),
    C,
}

pub type Age: u16;

pub type Data: (Age, String);

pub fn end() { // function is public
    process.exit(0);
}
