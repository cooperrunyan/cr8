use phf::phf_map;

pub static STD: phf::Map<&'static str, &'static str> = phf_map! {
    "<std>" => include_str!("std/mod.asm"),
    "<std>/arch" => include_str!("std/arch.asm"),
    "<std>/macro" => include_str!("std/macro/mod.asm"),
    "<std>/macro/math" => include_str!("std/macro/math/mod.asm"),
    "<std>/macro/math/add" => include_str!("std/macro/math/add.asm"),
    "<std>/macro/math/sub" => include_str!("std/macro/math/sub.asm"),
    "<std>/macro/call" => include_str!("std/macro/call.asm"),
    "<std>/macro/clear" => include_str!("std/macro/clear.asm"),
    "<std>/macro/jmp" => include_str!("std/macro/jmp.asm"),
    "<std>/macro/logic" => include_str!("std/macro/logic.asm"),
    "<std>/macro/send" => include_str!("std/macro/send.asm"),
    "<std>/math" => include_str!("std/math/mod.asm"),
    "<std>/math/mul" => include_str!("std/math/mul/mod.asm"),
    "<std>/math/mul/mul" => include_str!("std/math/mul/mul.asm"),
    "<std>/math/mul/mul16" => include_str!("std/math/mul/mul16.asm"),
    "<std>/math/shift" => include_str!("std/math/shift/mod.asm"),
    "<std>/math/shift/lsh" => include_str!("std/math/shift/lsh.asm"),
    "<std>/math/shift/lsh16" => include_str!("std/math/shift/lsh16.asm"),
};
