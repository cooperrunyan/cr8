use std::num::Wrapping;

use cfg::{
    mem::{PROGRAM_COUNTER, STACK, STACK_END, STACK_POINTER},
    reg::Register,
};

use crate::device::Device;

fn join((l, h): (u8, u8)) -> u16 {
    ((h as u16) << 8) | (l as u16)
}

fn split(hl: u16) -> (u8, u8) {
    ((hl as u8), (hl >> 8) as u8)
}

pub struct CR8 {
    pub reg: [u8; 8],
    pub mem: [u8; 65536],
    pub dev: Vec<Device>,
    pub speed: u64,
}

#[allow(dead_code)]
impl CR8 {
    pub fn new() -> Self {
        let mut cr8 = Self {
            reg: [0; 8],
            mem: [0; 65536],
            speed: 500,
            dev: vec![],
        };

        // initialize stack pointer;
        cr8.set_sp(split(STACK));
        cr8
    }

    pub fn speed(self, speed: u64) -> Self {
        Self {
            reg: self.reg,
            mem: self.mem,
            dev: self.dev,
            speed,
        }
    }

    fn hl(&self) -> (u8, u8) {
        let l = self.reg[Register::L as usize];
        let h = self.reg[Register::H as usize];

        (l, h)
    }

    fn sp(&self) -> (u8, u8) {
        let spl = self.mem[STACK_POINTER as usize];
        let sph = self.mem[(STACK_POINTER + 1) as usize];

        (spl, sph)
    }

    pub fn pc(&self) -> (u8, u8) {
        let pcl = self.mem[PROGRAM_COUNTER as usize];
        let pch = self.mem[(PROGRAM_COUNTER + 1) as usize];

        (pcl, pch)
    }

    fn set_sp(&mut self, (l, h): (u8, u8)) {
        self.mem[STACK_POINTER as usize] = l;
        self.mem[(STACK_POINTER + 1) as usize] = h;
    }

    pub fn lw_imm16(&mut self, to: Register, i: (u8, u8)) {
        let addr = join(i);
        println!("LW {to:#?}, {addr:#?}");
        self.reg[to as usize] = self.mem[addr as usize];
    }

    pub fn lw_hl(&mut self, to: Register) {
        let addr = join(self.hl());
        println!("LW {to:#?}, {}", addr);
        self.reg[to as usize] = self.mem[addr as usize];
    }

    pub fn sw_hl(&mut self, from: Register) {
        println!("SW {from:#?}, {}", join(self.hl()));
        self.mem[join(self.hl()) as usize] = self.reg[from as usize];
    }

    pub fn sw_imm16(&mut self, from: Register, i: (u8, u8)) {
        println!("SW {from:#?}, {}", join(i));
        self.mem[join(i) as usize] = self.reg[from as usize];
    }

    pub fn mov_reg(&mut self, to: Register, from: Register) {
        println!("MOV {to:#?}, {from:#?}");

        self.reg[to as usize] = self.reg[from as usize];
    }

    pub fn mov_imm8(&mut self, to: Register, imm8: u8) {
        println!("MOV {to:#?}, {imm8:#?}");
        self.reg[to as usize] = imm8;
    }

    pub fn push_imm8(&mut self, imm8: u8) {
        println!("PUSH {imm8:#?}");

        let sptr = join(self.sp());

        if sptr >= STACK_END {
            panic!("Stack overflow");
        }

        self.set_sp(split(sptr + 1));

        self.mem[(sptr + 1) as usize] = imm8;
    }

    pub fn push_reg(&mut self, reg: Register) {
        self.push_imm8(self.reg[reg as usize]);
    }

    pub fn pop(&mut self, reg: Register) {
        println!("POP {reg:#?}");
        let sptr = join(self.sp());

        if sptr <= STACK {
            panic!("Cannot pop empty stack");
        }

        self.reg[reg as usize] = self.mem[sptr as usize].clone();
        self.mem[sptr as usize] = 0;

        self.set_sp(split(sptr - 1));
    }

    pub fn jnz_imm8(&mut self, imm8: u8) {
        if imm8 == 0 {
            return;
        }

        self.mem[PROGRAM_COUNTER as usize] = self.reg[Register::L as usize];
        self.mem[(PROGRAM_COUNTER + 1) as usize] = self.reg[Register::H as usize];

        println!("JNZ {}, {imm8:#?}", join(self.pc()));
    }

    pub fn jnz_reg(&mut self, reg: Register) {
        self.jnz_imm8(self.reg[reg as usize]);
    }

    pub fn in_imm8(&mut self, into: Register, port: u8) {
        println!("IN {into:#?}, {port:#?}");
        let i = (|| {
            for (i, dev) in self.dev.iter().enumerate() {
                if dev.id == port {
                    return i;
                }
            }
            panic!("Attempted to address unpresent device");
        })();

        self.reg[into as usize] = self.dev[i].send.call((&self.dev[i], self));
    }

    pub fn in_reg(&mut self, into: Register, port: Register) {
        self.in_imm8(into, self.reg[port as usize]);
    }

    pub fn out_imm8(&mut self, send: Register, port: u8) {
        println!("OUT {send:#?}, {port:#?}");
        let i = (|| {
            for (i, dev) in self.dev.iter().enumerate() {
                if dev.id == port {
                    return i;
                }
            }
            panic!("Attempted to address unpresent device");
        })();

        self.dev[i]
            .recieve
            .call((&self.dev[i], self, self.reg[send as usize]));
    }

    pub fn out_reg(&mut self, send: Register, port: Register) {
        self.out_imm8(send, self.reg[port as usize]);
    }

    pub fn cmp_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("CMP {lhs:#?}, {imm8:#?}");

        let diff = (self.reg[lhs as usize] as i16) - (imm8 as i16);
        let mut f = 0;

        if diff == 0 {
            f = f | 0b0010;
        }

        if diff < 0 {
            f = f | 0b0001;
        }

        self.reg[Register::F as usize] = f;
    }

    pub fn cmp_reg(&mut self, lhs: Register, reg: Register) {
        self.cmp_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn adc_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("ADC {lhs:#?}, {imm8:#?}");

        let f = self.reg[Register::F as usize];
        let cf = (f >> 2) & 1;

        let res = Wrapping(self.reg[lhs as usize]) + Wrapping(imm8) + Wrapping(cf);
        let res = res.0;

        if res < self.reg[lhs as usize] || res < imm8 || res < cf {
            self.reg[Register::F as usize] = self.reg[Register::F as usize] | 0b0100;
        }

        self.reg[lhs as usize] = res;
    }

    pub fn adc_reg(&mut self, lhs: Register, reg: Register) {
        self.adc_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn sbb_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("SBB {lhs:#?}, {imm8:#?}");

        let f = self.reg[Register::F as usize];
        let bf = (f >> 3) & 1;

        let res = Wrapping(self.reg[lhs as usize]) + (Wrapping(!imm8 + 1) - Wrapping(bf));
        let res = res.0;

        if res > self.reg[lhs as usize] {
            self.reg[Register::F as usize] = 0b1000;
        }

        self.reg[lhs as usize] = res;
    }

    pub fn sbb_reg(&mut self, lhs: Register, reg: Register) {
        self.sbb_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn or_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("OR {lhs:#?}, {imm8:#?}");
        self.reg[lhs as usize] = self.reg[lhs as usize] | imm8;
    }

    pub fn or_reg(&mut self, lhs: Register, reg: Register) {
        self.or_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn nor_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("NOR {lhs:#?}, {imm8:#?}");
        self.reg[lhs as usize] = !(self.reg[lhs as usize] | imm8);
    }

    pub fn nor_reg(&mut self, lhs: Register, reg: Register) {
        self.nor_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn and_imm8(&mut self, lhs: Register, imm8: u8) {
        println!("AND {lhs:#?}, {imm8:#?}");
        self.reg[lhs as usize] = self.reg[lhs as usize] & imm8;
    }

    pub fn and_reg(&mut self, lhs: Register, reg: Register) {
        self.and_imm8(lhs, self.reg[reg as usize]);
    }

    pub fn dev_add(&mut self, dev: Device) {
        let mut dev = dev;
        dev.id = self.dev.len() as u8;
        self.dev.push(dev);
    }

    pub fn dev_rm(&mut self, id: u8) {
        let i = (|| {
            for (i, dev) in self.dev.iter().enumerate() {
                if dev.id == id {
                    return i;
                }
            }
            panic!("Attempted to remove unpresent device");
        })();

        let _ = self.dev[i].drop.call((&self.dev[i], self));
        self.dev.remove(i);
    }

    pub fn debug(&self) {
        println!("A: {}", self.reg[Register::A as usize]);
        println!("B: {}", self.reg[Register::B as usize]);
        println!("C: {}", self.reg[Register::C as usize]);
        println!("D: {}", self.reg[Register::D as usize]);
        println!("Z: {}", self.reg[Register::Z as usize]);
        println!("HL: {}", join(self.hl()));
        println!("[HL]: {}", self.mem[join(self.hl()) as usize]);
        println!("SP: {}", join(self.sp()) - STACK);
        println!("[SP]: {}", self.mem[join(self.sp()) as usize]);
        println!();
        println!("Devices:");

        for (i, dev) in self.dev.iter().enumerate() {
            println!("  {i}: {}", dev.name);
        }

        println!();
        let f = self.reg[Register::F as usize];
        let lf = f & 1;
        let ef = (f >> 1) & 1;
        let cf = (f >> 2) & 1;
        let zf = (f >> 3) & 1;

        println!();
        println!("LF: {}", lf == 1);
        println!("EF: {}", ef == 1);
        println!("CF: {}", cf == 1);
        println!("ZF: {}", zf == 1);
    }

    pub fn tick_pc(&mut self) {
        let pc = join(self.pc());
        let (pcl, pch) = split(pc + 1);
        self.mem[PROGRAM_COUNTER as usize] = pcl;
        self.mem[(PROGRAM_COUNTER + 1) as usize] = pch;
    }
}
