```c
fn _f0 (args i32:a, i32:b) i32 {
// Locals:
// _0 = return value
// _1 = a
// _2 = b
// _3 = Point {x, y}
// _4 = &mut _3.x
// _5 = & _3
// _6 = array [i32; 3]
// _7 = &mut _6[1]
// _8 = temporary
// _9 = bool

bb0:
    // copy args into locals (optional, but common)
    move _1, P:a // move _l:a into _1
    move _2, P:b

    // struct creation^
    assign _3.x, _1
    assign _3.y, _2

    // immutable borrow of whole struct
    borrow _5, ref _3

    // mutable borrow of field
    borrowm _4, mref _3.x
    assign dref _4, (bin:add dref _4, 10)

    // read via immutable borrow
    assign _8, load _5.x

    // array creation
    // alloc on 'stack' or 'heap'
    assign _6, alloc [i32; 3]
    assign _6[0], 1
    assign _6[1], 2
    assign _6[2], 3

    // borrow element
    borrowm _7, mref _6[1]
    assign dref _7, (bin:mul dref _7, 5)

    // condition
    assign _9, (cmp:gt _8, 0)
    _9 ? jmp bb1, jmp bb2

bb1:
    // call a function (e.g. _f1)
    assign _0, call _f1 (args _8, _6[1])

    // jump to label
    jmp bb4

bb2:
    // loop-like control flow
    assign _0, 0
    jmp bb3

bb3:
    // loop body (simplified)
    assign _6[0], (bin:add _6[0], 1)
    assign _6[2], (bin:sub _6[2], 1)

    // break condition
    assign _9, (cmp:eq _6[2], 0)
    _9 ? jmp bb4, jmp bb3

bb4:
    // drop mutable borrow (implicit in NLL, but explicit here)
    dropm _4

    // drop immutable borrow
    drop _5

    // return
    ret _0
}

fn _f1 (args i32:a, i32:b) i32 {
// _0 = return value
// _1 = a
// _2 = b

bb0:
    // moving values
    move _1, P:a
    move _2, P:b

    // addition
    assign _0, (bin:add _1, _2)
    ret _0
}

!begin unsafe
assign _3, dref _rawptr
!end unsafe
```
