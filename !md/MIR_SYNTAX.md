fn _f0 (args i32:a, i32:b) i32 {
  bb0:
    // copy args into locals (optional, but common)
    move $0, P:a // move _l:a into _1
    move $0, P:b

    // struct creation^
    assign $3.x, $1
    assign $3.y, $2

    // immutable borrow of whole struct
    borrow $5, ref $3

    // mutable borrow of field
    borrowm $4, mref $3.x
    assign dref $4, (bin:add dref $4, 0xA)

    // read via immutable borrow
    assign $8, load $5.x

    // array creation
    // alloc on 'stack' or 'heap'
    assign $6, alloc [i32; 0x3]
    assign $6[0x0], 0x1
    assign $6[0x1], 0x2
    assign $6[0x2], 0x3

    // borrow element
    borrowm $7, mref $6[0x1]
    assign dref $7, (bin:mul dref $7, 0x5)

    // condition
    assign $9, (cmp:gt $8, 0x0)
    $9 ? jmp bb1, jmp bb2

  bb1:
    // call a function (e.g. _f1)
    assign $0, call $f1 (args $8, $6[0x1])

    // jump to label
    jmp bb4

  bb2:
    // loop-like control flow
    assign $0, 0x0
    jmp bb3

  bb3:
    // loop body (simplified)
    assign $6[0x0], (bin:add $6[0x0], 0x1)
    assign $6[0x2], (bin:sub $6[0x2], 0x1)

    // break condition
    assign $9, (cmp:eq $6[0x2], 0x0)
    $9 ? jmp bb4, jmp bb3

  bb4:
    // drop mutable borrow (implicit in NLL, but explicit here)
    dropm $4

    // drop immutable borrow
    drop $5

    // return
    ret $0
}

fn _f1 (args i32:a, i32:b) i32 {
// _0 = return value
// _1 = a
// _2 = b

bb0:
    // moving values
    move $1, P:a
    move $2, P:b

    // addition
    assign $0, (bin:add $1, $2)
    ret $0
}

!begin unsafe
assign $3, dref $rawptr
!end unsafe
