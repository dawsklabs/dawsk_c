```c
get sys::prc; // imports a module

dec m: i8 = -15;

dec mut x: f32 = 18e-1;

dec publy t = 4 - 0.3;

dec publy mut z: f32 = x + t; // first 'publy' and afterwards 'mut'

dec b = &z;

struct MyStruct {
    a: i8 publy,
    b: u16 publy,
    c: str,
    x: f32,
    t: Vec<u8> publy,
} publy; // 'publy' at the end due to readability and just overall language design

impl MyStruct {
    func @overload(operator::add)(&self, other: MyStruct) -> Self { // the name is '@overload(operator::add)' which will trigger a point in den the parser to overload the operator
        dec v: Vec<u8> = self.t.clone();
        v.extend(other.t);
        Self {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c,
            x: self.x + other.x,
            t: v,
        }
    }
}

enum Type {
    A(i32),
    C,
}

enum MyEnum {
    A(Type),
    C,
} publy;

type Age: u16 publy; // ':' cuz its not really a assignment but more like its own declaration syntax
type Data: (Age, str) publy;

func end() { // function returns nothing... (void)
    prc::exit(0);
}
```
