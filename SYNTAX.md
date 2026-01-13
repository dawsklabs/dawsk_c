# Syntax

```c
get sys::prc; // imports a module

dec(const) m: u8 = b'e'; // constants
dec m: i8 = -15; // variables

dec mut x: f32 = 18e-1;

dec pub t: f32 = 4 - 0.3;

dec pub mut z: f32 = x + t; // first 'pub', 'mut' afterwards

dec c = &(&mut z);  // c: &(&mut f32) (explizit); ref takes the same type (should be logical) but as ref...

dec mut y: = *(*c); // y should then have the value of z

dec(struct) pub MyStruct {
    pub a: i8,
    pub b: u16,
    c: String,
    x: f32,
    pub t: Vec<u8>,
};

impl MyStruct {
    func @overload(operator::add)(&self, other: MyStruct) -> Self {
        dec v: Vec<u8> = self.t.clone();
        v.extend(other.t);
        Self {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c.clone(),
            x: self.x + other.x,
            t: v,
        }
    }
}

dec(trait) pub Printable {
    func print(&self);
}

impl(Printable) MyStruct {
    func print(&self) {
        prc::println("MyStruct values");
    }
}

dec(enum) Type {
    A(i32),
    C,
}

dec(enum) pub MyEnum {
    A(Type),
    C,
};

dec(type) pub Age(u16);

dec(type) pub Data(Age, String);

func pub end() { // function is public
    prc::exit(0);
}
```
