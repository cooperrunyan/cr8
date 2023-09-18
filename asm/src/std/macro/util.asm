;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
; <std>/macro/util

#macro mov16 {
    ($inlo: reg, $inhi: reg, $frlo: reg | imm8, $frhi: reg | imm8) => {
        mov $inlo, $frlo
        mov $inhi, $frhi
    }
    ($inlo: reg, $inhi: reg, $from: imm16) => {
        mov $inlo, $from.l
        mov $inhi, $from.h
    }
}

#macro swi {
    ($to: imm16, $b: imm8) => {
        mov %f, $b
        sw $to, %f
    }
}
