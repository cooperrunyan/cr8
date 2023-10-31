#[use(std)]

#[main]
main:
  bank 1

  .loop:
    test 0, 0
    test 29, 0
    test 30, 0
    test 31, 0
    test 1, 1
    jmp .loop

#[macro] test: {
  ($a: lit, $b: lit) => {
    mov %a, $a
    mov %b, $b
    call filled_box
  }
}