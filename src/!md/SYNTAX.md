# Syntax

```rs
dec(const) m: u8 = b'e'; // constants
dec m: i8 = -15; // variables

dec mut x: f32 = 18e-1;

dec(pub) t: f32 = 4 - 0.3;

dec(pub) mut z: f32 = x + t; // first 'pub', 'mut' afterwards

dec c = &&mut z;  // c: &&mut f32 (explizit); ref takes the same type (should be logical) but as ref...

dec mut y: = **c; // y should then have the value of z

dec(pub struct) MyStruct {
    pub a: i8,
    pub b: u16,
    c: String,
    x: f32,
    pub t: DynArr<u8>,
}

impl Add for MyStruct {
    func add(self, )
}

dec(pub trait) Printable {
    func print(&self);
}

impl Printable for MyStruct {
    func print(&self) {
        println!("MyStruct values");
    }
}

dec(pub enum) Type {
    A(i32),
    C,
}

dec(pub enum) MyEnum {
    A(Type),
    C,
}

dec(pub type) Age: u16;

dec(pub type) Data: (Age, String);

func(pub) end() { // function is public
    prc::exit(0);
}
```
