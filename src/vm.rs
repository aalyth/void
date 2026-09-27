use core::panic;
use std::rc::Rc;

use crate::code::{Bytecode, Constant};

pub struct Function {
    pub arity: usize,
    pub locals: usize,
    pub code: Vec<u8>,
}

struct CallFrame {
    prev_pc: u64,
    stack_len: u64,
    prev_fn: u32,
}

pub struct Vm {
    pc: usize,
    pub stack: Vec<u64>,
    func: Rc<Function>,
    func_stack_offset: usize,
    func_id: u32,

    functions: Vec<Rc<Function>>,
    constants: Vec<Constant>,
}

impl CallFrame {
    const WORDS: usize = 3;
}

impl From<CallFrame> for [u64; CallFrame::WORDS] {
    fn from(frame: CallFrame) -> Self {
        [frame.prev_pc, frame.stack_len, frame.prev_fn as u64]
    }
}

impl From<[u64; CallFrame::WORDS]> for CallFrame {
    fn from(value: [u64; CallFrame::WORDS]) -> Self {
        Self {
            prev_pc: value[0],
            stack_len: value[1],
            prev_fn: value[2] as u32,
        }
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: Vec::new(),
            func: Rc::new(Function {
                arity: 0,
                locals: 0,
                code: Vec::new(),
            }),
            func_stack_offset: 0,
            func_id: 0,
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn add_function(&mut self, func: Function) {
        self.functions.push(Rc::new(func));
    }

    pub fn add_constant(&mut self, cons: Constant) {
        self.constants.push(cons);
    }

    pub fn execute(&mut self, func_id: usize) {
        let Some(function) = self.functions.get(func_id) else {
            panic!("Unknowon function id: {}", func_id);
        };
        self.set_func_as_current(function.clone(), func_id);
        while self.pc < self.func.code.len() {
            let instr = Bytecode::from_byte_unchecked(self.get_byte_at(self.pc));
            // println!("instr: {:<30?}, stack: {:?}", instr, self.stack);
            self.execute_instr(instr);
        }
    }

    fn execute_instr(&mut self, instr: Bytecode) {
        match instr {
            Bytecode::Nop => self.pc += 1,

            Bytecode::Const => {
                let id = self.get_byte_at(self.pc + 1) as usize;
                let cons = unsafe { *self.constants.get_unchecked(id) };
                self.stack.push(cons.into());
                self.pc += 2;
            }
            Bytecode::Const2 => {
                let id = unsafe { read_u16_unchecked(&self.func.code, self.pc + 1) } as usize;
                let cons = unsafe { *self.constants.get_unchecked(id) };
                self.stack.push(cons.into());
                self.pc += 3;
            }

            Bytecode::PushLocal => {
                let id = self.get_byte_at(self.pc + 1) as usize;
                let val = unsafe { *self.stack.get_unchecked(self.func_stack_offset + id) };
                self.stack.push(val);
                self.pc += 2;
            }
            Bytecode::StoreLocal => {
                let id = self.get_byte_at(self.pc + 1) as usize;
                let val = unsafe { pop_unchecked(&mut self.stack) };
                self.stack[self.func_stack_offset + id] = val;
                self.pc += 2;
            }

            Bytecode::Addi => {
                let (left, right) = unsafe { pop2_unchecked(&mut self.stack) };
                let res = (left as i64) + (right as i64);
                self.stack.push(res as u64);
                self.pc += 1;
            }
            Bytecode::Subi => {
                let (left, right) = unsafe { pop2_unchecked(&mut self.stack) };
                let res = (left as i64) - (right as i64);
                self.stack.push(res as u64);
                self.pc += 1;
            }

            Bytecode::JmpIfLei => {
                let (left, right) = unsafe { pop2_unchecked(&mut self.stack) };
                if (left as i64) <= (right as i64) {
                    self.pc += 1;
                    let offset = unsafe { read_u32_unchecked(&self.func.code, self.pc) } as i32;
                    self.pc = ((self.pc as isize) + (offset as isize)) as usize;
                } else {
                    self.pc += 5;
                }
            }

            Bytecode::Call => {
                let id = unsafe { read_u16_unchecked(&self.func.code, self.pc + 1) } as usize;
                let func = unsafe { self.functions.get_unchecked(id) }.clone();
                for _ in 0..func.locals {
                    self.stack.push(0);
                }

                let frame = CallFrame {
                    prev_pc: (self.pc + 3) as u64,
                    stack_len: self.func_stack_offset as u64,
                    prev_fn: self.func_id,
                };
                self.set_func_as_current(func, id);

                let raw_frame: [u64; CallFrame::WORDS] = frame.into();
                self.stack.extend(raw_frame);
            }

            Bytecode::Return => {
                let result = unsafe { pop_unchecked(&mut self.stack) };
                let frame =
                    unsafe { read_frame_at(&self.stack, self.stack.len() - CallFrame::WORDS) };
                let offset =
                    self.stack.len() - (self.func.arity + self.func.locals) - CallFrame::WORDS;
                self.stack.truncate(offset);
                self.stack.push(result);

                self.pc = frame.prev_pc as usize;
                self.func = unsafe { self.functions.get_unchecked(frame.prev_fn as usize) }.clone();
                self.func_stack_offset = frame.stack_len as usize;
                self.func_id = frame.prev_fn;
            }

            Bytecode::Halt => {
                self.pc = self.func.code.len();
            }
        }
    }

    fn get_byte_at(&self, offset: usize) -> u8 {
        unsafe { *self.func.code.get_unchecked(offset) }
    }

    fn set_func_as_current(&mut self, func: Rc<Function>, id: usize) {
        self.pc = 0;
        // TODO: u32 or u16
        self.func_id = id as u32;
        self.func_stack_offset = self.stack.len() - (func.arity + func.locals);
        self.func = func;
    }
}

// SAFETY: caller guarantees offset + 2 <= buf.len()
unsafe fn read_u16_unchecked(buf: &[u8], offset: usize) -> u16 {
    let ptr = buf.as_ptr().add(offset) as *const [u8; 2];
    u16::from_le_bytes(ptr.read_unaligned())
}

// SAFETY: caller guarantees offset + 4 <= buf.len()
unsafe fn read_u32_unchecked(buf: &[u8], offset: usize) -> u32 {
    let ptr = buf.as_ptr().add(offset) as *const [u8; 4];
    u32::from_le_bytes(ptr.read_unaligned())
}

/// SAFETY: caller guarantees offset + 24 <= buf.len()
pub unsafe fn read_frame_at(buf: &[u64], offset: usize) -> CallFrame {
    let ptr = buf.as_ptr().add(offset) as *const [u64; CallFrame::WORDS];
    ptr.read().into()
}

#[inline]
unsafe fn pop_unchecked<T>(v: &mut Vec<T>) -> T {
    debug_assert!(!v.is_empty());
    let new_len = v.len() - 1;
    v.set_len(new_len);
    std::ptr::read(v.as_ptr().add(new_len))
}

#[inline]
unsafe fn pop2_unchecked<T>(v: &mut Vec<T>) -> (T, T) {
    let new_len = v.len() - 2;
    v.set_len(new_len);
    let ptr = v.as_ptr().add(new_len);
    (std::ptr::read(ptr), std::ptr::read(ptr.add(1)))
}
