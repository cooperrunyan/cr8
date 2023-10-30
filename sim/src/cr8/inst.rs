use asm::op::{Operation, OperationArgAmt};
use log::trace;
use std::sync::RwLock;

use anyhow::{anyhow, bail, Result};
use asm::reg::Register;

use crate::cr8::Joinable;
use crate::devices::Devices;

use super::mem::Mem;
use super::{CR8, STACK, STACK_END};
use Operation as O;
use OperationArgAmt as A;

impl CR8 {
    /// Decide which [Operation] to run
    pub fn delegate(
        &mut self,
        mem: &RwLock<Mem>,
        dev: &RwLock<Devices>,
        bytes: [u8; 4],
    ) -> Result<u8> {
        let instruction =
            Operation::try_from(bytes[0] >> 2).map_err(|_| anyhow!("Invalid operation"))?;
        let amt = OperationArgAmt::from(bytes[0]);

        match instruction {
            O::MOV => self.mov(amt, bytes),
            O::JNZ => self.jnz(amt, bytes),
            O::LW => self.lw(mem, amt, bytes),
            O::SW => self.sw(mem, amt, bytes),
            O::PUSH => self.push(mem, amt, bytes),
            O::POP => self.pop(mem, amt, bytes),
            O::IN => self.r#in(dev, amt, bytes),
            O::OUT => self.out(dev, amt, bytes),
            O::ADC => self.add(amt, bytes),
            O::SBB => self.sub(amt, bytes),
            O::CMP => self.cmp(amt, bytes),
            O::AND => self.and(amt, bytes),
            O::OR => self.or(amt, bytes, false),
            O::NOR => self.or(amt, bytes, true),
            O::BANK => self.bank(mem, amt, bytes),
        }
    }

    /// LW: (see README.md)
    fn lw(&mut self, mem: &RwLock<Mem>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (to, addr, sz) = match amt {
            A::R1I0 => (bytes[1] & 0b1111, self.xy(), 2),
            A::R1I1 => (bytes[1] & 0b1111, (bytes[2], bytes[3]).join(), 4),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: LW {to:#?} {addr:04x}", self.pc);
        self.reg[to as usize] = {
            let mem = mem.read().unwrap();
            mem.get(addr)?
        };
        Ok(sz)
    }

    /// SW: (see README.md)
    fn sw(&mut self, mem: &RwLock<Mem>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (val, addr, sz) = match amt {
            A::R1I0 => (self.reg[(bytes[1] & 0b1111) as usize], self.xy(), 2),
            A::R1I1 => (
                self.reg[(bytes[1] & 0b1111) as usize],
                (bytes[2], bytes[3]).join(),
                4,
            ),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: SW {val:#?} {addr:04x}", self.pc);
        let mut mem = mem.write().unwrap();
        mem.set(addr, val)?;
        Ok(sz)
    }

    /// MOV: (see README.md)
    fn mov(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (into, val, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: MOV {into:#?}, {val:02x} | {val:?}", self.pc);
        self.reg[into as usize] = val;
        Ok(sz)
    }

    /// PUSH: (see README.md)
    fn push(&mut self, mem: &RwLock<Mem>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        if self.sp >= STACK_END {
            bail!("Stack overflow");
        }

        self.sp += 1;

        let (val, sz) = match amt {
            A::R1I0 => (self.reg[(bytes[1] & 0b1111) as usize], 2),
            A::R0I1 => (bytes[1], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        {
            let mut mem = mem.write().unwrap();
            mem.set(self.sp, val)?;
        };

        trace!(
            "{:04x}: PUSHED: [{:04x}] {:02x}",
            self.pc,
            self.sp as i128 - STACK as i128,
            val,
        );
        Ok(sz)
    }

    /// POP: (see README.md)
    fn pop(&mut self, mem: &RwLock<Mem>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        if self.sp < STACK {
            bail!("Cannot pop empty stack");
        }

        let reg = match amt {
            A::R1I0 => bytes[1] & 0b1111,
            _ => bail!("Invalid amount {amt:?}"),
        };

        {
            let mut mem = mem.write().unwrap();
            self.reg[reg as usize] = mem.get(self.sp)?;
            mem.set(self.sp, 0)?;
        };

        trace!(
            "{:04x}: POPPED: [{:04x}] {:?}",
            self.pc,
            self.sp - STACK,
            reg,
        );

        self.sp -= 1;
        Ok(2)
    }

    /// JNZ: (see README.md)
    fn jnz(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (condition, sz) = match amt {
            A::R1I0 => (self.reg[(bytes[1] & 0b1111) as usize], 2),
            A::R0I1 => (1, 1),
            _ => bail!("Invalid amount {amt:?}"),
        };
        if condition == 0 {
            trace!("{:04x}: No JNZ", self.pc);
            return Ok(sz);
        }

        let old = self.pc;

        self.pc = self.xy();

        trace!("{:04x}: JNZ to {:04x}", old, self.pc);
        Ok(0)
    }

    /// IN: (see README.md)
    fn r#in(&mut self, dev: &RwLock<Devices>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (into, port, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: IN {into:#?}, {port:02x}", self.pc);
        let mut devices = dev.write().unwrap();
        self.reg[into as usize] = devices.receive(port)?;
        Ok(sz)
    }

    /// OUT: (see README.md)
    fn out(&mut self, dev: &RwLock<Devices>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (send, port, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: OUT {send:#?}, {port:02x}", self.pc);
        let mut devices = dev.write().unwrap();
        devices.send(port, self.reg[send as usize])?;
        Ok(sz)
    }

    /// CMP: (see README.md)
    fn cmp(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (lhs, rhs, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: CMP {lhs:#?}, {rhs:02x}", self.pc);

        let diff = (self.reg[lhs as usize] as i16) - (rhs as i16);
        let mut f = 0;

        if diff == 0 {
            f |= 0b0010;
        }

        if diff < 0 {
            f |= 0b0001;
        }

        self.reg[Register::F as usize] = f;
        Ok(sz)
    }

    /// ADD: (see README.md)
    fn add(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (lhs, rhs, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: ADD {lhs:#?}, {rhs:02x}", self.pc);

        let f = self.reg[Register::F as usize];
        let cf = (f >> 2) & 1;

        let (res, carry) = self.reg[lhs as usize].carrying_add(rhs, cf == 1);

        if carry {
            self.reg[Register::F as usize] |= 0b0100;
        } else {
            self.reg[Register::F as usize] &= 0b1011;
        }

        self.reg[lhs as usize] = res;
        Ok(sz)
    }

    /// SUB: (see README.md)
    fn sub(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (lhs, rhs, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: SUB {lhs:#?}, {rhs:02x}", self.pc);

        let f = self.reg[Register::F as usize];
        let bf = (f >> 3) & 1;

        let (res, borrow) = self.reg[lhs as usize].borrowing_sub(rhs, bf == 1);

        if borrow {
            self.reg[Register::F as usize] |= 0b1000;
        } else {
            self.reg[Register::F as usize] &= 0b0111;
        }

        self.reg[lhs as usize] = res;
        Ok(sz)
    }

    /// OR: (see README.md)
    fn or(&mut self, amt: OperationArgAmt, bytes: [u8; 4], not: bool) -> Result<u8> {
        let (lhs, rhs, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: OR {lhs:#?}, {rhs:02x}", self.pc);
        self.reg[lhs as usize] |= rhs;
        if not {
            self.reg[lhs as usize] = !self.reg[lhs as usize];
        }
        Ok(sz)
    }

    /// AND: (see README.md)
    fn and(&mut self, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (lhs, rhs, sz) = match amt {
            A::R1I1 => (bytes[1] & 0b1111, bytes[2], 3),
            A::R2I0 => (bytes[1] & 0b1111, self.reg[(bytes[1] >> 4) as usize], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: AND {lhs:#?}, {rhs:02x}", self.pc);
        self.reg[lhs as usize] &= rhs;
        Ok(sz)
    }

    /// MOV: (see README.md)
    fn bank(&mut self, mem: &RwLock<Mem>, amt: OperationArgAmt, bytes: [u8; 4]) -> Result<u8> {
        let (val, sz) = match amt {
            A::R1I0 => (bytes[1] & 0b1111, 2),
            A::R0I1 => (bytes[1], 2),
            _ => bail!("Invalid amount {amt:?}"),
        };
        trace!("{:04x}: BANK {val:02x} | {val:?}", self.pc);
        mem.write().unwrap().select(val)?;
        Ok(sz)
    }
}
