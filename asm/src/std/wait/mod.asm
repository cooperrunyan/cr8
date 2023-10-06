; Can be shortcut called with the macro: `wait [TICKS]`

#[macro] wait: {
    ($tickamt: imm16) => {
        mov %b, $tickamt.h
        mov %a, $tickamt.l
        call [_wait]
    }
}

_wait:
    .loop:
        dec %a, %b
        jnz [.loop], %a
        jnz [.loop], %b
        ret
