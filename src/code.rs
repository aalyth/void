use std::collections::HashMap;

use thiserror::Error;

const MAX_LOCAL_VARS: usize = 255;
const MAX_CONSTANTS: usize = 1 << 16;

#[derive(Debug)]
#[repr(u8)]
pub enum Bytecode {
    Nop = 0,

    Const,
    Const2,

    PushLocal,
    StoreLocal,

    Addi,
    Subi,

    JmpIfLei,

    Call,
    Return,
    Halt,
}

pub enum Instruction {
    Constant(Constant),

    GetArgument(String),
    GetLocal(String),

    Add(Type),
    Sub(Type),

    Jmp(Jump, Type, String),
    Label(String),

    CallFunction(String),
    Return,
}

pub struct ProgramBuilder {
    functions: HashMap<String, Vec<Instruction>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Constant {
    Int(i64),
}

struct JumpLocation {
    bytecode_offset: usize,
    label: String,
}

pub enum Type {
    Int,
}

pub enum Jump {
    IfLe,
}

struct Variable {
    id: u8,
    ty: Type,
}

#[derive(Debug, Error)]
pub enum CodeError {
    #[error("local variable {name} already exists")]
    LocalVariableExists { name: String },

    #[error("the local variable {name} does not exist")]
    LocalVariableDoesNotExist { name: String },

    #[error("the maximum number of local variables ({MAX_LOCAL_VARS}) has been reached")]
    MaxLocalVariables,

    #[error("the maximum number of constants ({MAX_CONSTANTS}) has been reached")]
    MaxConstants,

    #[error("forward label jump {label} is not defined")]
    UnmatchedForwardLabelJump { label: String },

    #[error("instruction jump distance too large")]
    JumpDistanceTooLarge,

    #[error("label '{label}' already exists")]
    LabelAlreadyExists { label: String },
}

pub struct CodeBuilder {
    pub code: Vec<u8>,

    labels: HashMap<String, usize>,
    jump_locations: Vec<JumpLocation>,

    locals: HashMap<String, Variable>,
    constants: HashMap<Constant, u16>,
}

pub struct FunctionBuilder {
    arguments: HashMap<String, Variable>,
}

impl Bytecode {
    pub fn from_byte_unchecked(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<Constant> for u64 {
    fn from(cons: Constant) -> Self {
        match cons {
            Constant::Int(val) => val as u64,
        }
    }
}

impl CodeBuilder {
    pub fn new() -> Self {
        CodeBuilder {
            code: Vec::new(),
            labels: HashMap::new(),
            jump_locations: Vec::new(),
            locals: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    pub fn declare_local(mut self, name: String, ty: Type) -> Result<Self, CodeError> {
        if self.locals.contains_key(&name) {
            return Err(CodeError::LocalVariableExists { name });
        }

        if self.locals.len() >= MAX_LOCAL_VARS {
            return Err(CodeError::MaxLocalVariables);
        }

        let var = Variable {
            id: self.locals.len() as u8,
            ty,
        };
        self.locals.insert(name, var);
        Ok(self)
    }

    pub fn get_local(mut self, name: String) -> Result<Self, CodeError> {
        let Some(var_id) = self.locals.get(&name).map(|var| var.id) else {
            return Err(CodeError::LocalVariableDoesNotExist { name });
        };
        self.push_instr(Bytecode::PushLocal);
        self.code.push(var_id);
        Ok(self)
    }

    pub fn push_constant(mut self, cons: Constant) -> Result<Self, CodeError> {
        if !self.constants.contains_key(&cons) {
            if self.constants.len() >= MAX_CONSTANTS {
                return Err(CodeError::MaxConstants);
            }

            self.constants.insert(cons, self.constants.len() as u16);
        }
        // SAFETY: checked on the line before
        let const_id = *self.constants.get(&cons).unwrap();
        if let Ok(id) = u8::try_from(const_id) {
            self.push_instr(Bytecode::Const);
            self.code.push(id);
        } else {
            self.push_instr(Bytecode::Const2);
            self.code.extend(const_id.to_le_bytes());
        }
        Ok(self)
    }

    pub fn add(mut self, ty: Type) -> Self {
        match ty {
            Type::Int => self.push_instr(Bytecode::Addi),
        }
        self
    }

    pub fn sub(mut self, ty: Type) -> Self {
        match ty {
            Type::Int => self.push_instr(Bytecode::Subi),
        }
        self
    }

    pub fn jmp_if(self, jmp: Jump, label: String) -> Self {
        match jmp {
            Jump::IfLe => self.jump_to_label(Bytecode::JmpIfLei, label),
        }
    }

    pub fn instr_return(mut self) -> Self {
        self.push_instr(Bytecode::Return);
        self
    }

    pub fn set_label(mut self, label: String) -> Result<Self, CodeError> {
        if self.labels.contains_key(&label) {
            return Err(CodeError::LabelAlreadyExists { label });
        }
        self.labels.insert(label, self.code.len());
        Ok(self)
    }

    pub fn build(mut self) -> Result<Vec<u8>, CodeError> {
        for jump_location in self.jump_locations {
            let offset = jump_location.bytecode_offset;
            let Some(label_offset) = self.labels.get(&jump_location.label) else {
                return Err(CodeError::UnmatchedForwardLabelJump {
                    label: jump_location.label,
                });
            };

            let distance = *label_offset as isize - offset as isize;
            if let Ok(jmp) = i32::try_from(distance) {
                self.code[offset..offset + 4].copy_from_slice(&jmp.to_le_bytes());
            } else {
                return Err(CodeError::JumpDistanceTooLarge);
            }
        }

        Ok(self.code)
    }

    pub fn push_instr(&mut self, instr: Bytecode) {
        self.code.push(instr as u8);
    }

    fn jump_to_label(mut self, jmp_instr: Bytecode, label: String) -> Self {
        self.push_instr(jmp_instr);
        if let Some(offset) = self.labels.get(&label) {
            let distance = *offset as i32 - self.code.len() as i32;
            self.code.extend(distance.to_le_bytes());
        } else {
            self.jump_locations.push(JumpLocation {
                bytecode_offset: self.code.len(),
                label,
            });
            self.code.extend((0 as i32).to_le_bytes());
        }
        self
    }
}
