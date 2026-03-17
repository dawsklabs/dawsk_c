# Syntax

```rs
dec(const) m: u8 = b'e'; // constants
dec m: i8 = -15; // variables

dec mut x: f32 = 18e-1;

dec(pub) t: f32 = 4 - 0.3;

dec(pub) mut z: f32 = x + t; // first 'pub', 'mut' afterwards

dec c = &&mut z;  // c: &&mut f32 (explizit); ref takes the same type (should be logical) but as ref...

dec mut y: = **c; // y should then have the value of z

struct(pub) MyStruct {
    a(pub): i8,
    b(pub): u16,
    c(pub): String,
    x: f32,
    t(pub): DynArr<u8>,
}

impl Add for MyStruct {
    fn add(self, )
}

trait(pub) Printable {
    fn print(&self);
}

impl Printable for MyStruct {
    fn print(&self) {
        println!("MyStruct values");
    }
}

enum(pub) Type {
    A(i32),
    C,
}

enum(pub) MyEnum {
    A(Type),
    C,
}

type(pub) Age: u16;

type(pub) Data: (Age, String);

fn(pub) end() { // function is public
    prc::exit(0);
}
```
