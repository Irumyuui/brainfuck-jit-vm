use std::io::{Read, Write};

use dynasm::dynasm;
use dynasmrt::{DynasmApi, DynasmLabelApi};

use crate::{
    bfir::{self, BfIR},
    errors::{RuntimeError, VMError},
};

const MEMORY_SIZE: usize = 30000;

pub struct BfVM<'io> {
    code: dynasmrt::ExecutableBuffer,
    start: dynasmrt::AssemblyOffset,
    memory: Box<[u8]>,
    input: Box<dyn Read + 'io>, 
    output: Box<dyn Write + 'io>,
}

#[inline(always)]
fn vm_error(re: RuntimeError) -> *mut VMError {
    Box::into_raw(Box::new(VMError::from(re)))
}

impl<'io> BfVM<'io> {
    unsafe extern "sysv64" fn getbyte(this: *mut Self, ptr: *mut u8) -> *mut VMError {
        let mut buf = [0_u8];
        let this = &mut *this;

        match this.input.read(&mut buf) {
            Ok(0) => {}
            Ok(1) => *ptr = buf[0],
            Err(e) => return vm_error(RuntimeError::IO(e)),
            _ => unreachable!(),
        }

        std::ptr::null_mut()
    }

    unsafe extern "sysv64" fn putbyte(this: *mut Self, ptr: *const u8) -> *mut VMError {
        let buf = std::slice::from_ref(&*ptr);
        let this = &mut *this;

        match this.output.write_all(buf) {
            Ok(()) => std::ptr::null_mut(),
            Err(e) => vm_error(RuntimeError::IO(e)),
        }
    }

    unsafe extern "sysv64" fn overflow_error() -> *mut VMError {
        vm_error(RuntimeError::PointerOverflow)
    }
}

impl<'io> BfVM<'io> {
    pub fn new(
        src_code: &str,
        input: Box<dyn Read + 'io>,
        output: Box<dyn Write + 'io>,
        optimize: bool,
    ) -> Result<Self, VMError> {
        let mut ir = bfir::compile(&src_code)?;

        if optimize {
            bfir::optimize(&mut ir);
        }

        let (code, start) = compile(&ir)?;
        let memory = vec![0; MEMORY_SIZE].into_boxed_slice();

        Ok(Self {
            code,
            start,
            memory,
            input,
            output,
        })
    }

    pub fn run(&mut self) -> Result<(), VMError> {
        type RawFn = unsafe extern "sysv64" fn(
            this: *mut BfVM<'_>,
            memory_start: *mut u8,
            memory_end: *const u8,
        ) -> *mut VMError;

        let raw_fn: RawFn = unsafe { std::mem::transmute(self.code.ptr(self.start)) };
        let memory_start = self.memory.as_mut_ptr();
        let memory_end = unsafe { memory_start.add(MEMORY_SIZE) };

        let ret: *mut VMError = unsafe { raw_fn(self, memory_start, memory_end) };

        if ret.is_null() {
            Ok(())
        } else {
            Err(*unsafe { Box::from_raw(ret) })
        }
    }
}

fn compile(
    code: &[BfIR],
) -> Result<(dynasmrt::ExecutableBuffer, dynasmrt::AssemblyOffset), VMError> {
    let mut ops = dynasmrt::x64::Assembler::new()?;
    let start = ops.offset();

    let mut loops = vec![];

    dynasm!(ops
    ; push rax
    ; mov r12, rdi
    ; mov r13, rsi
    ; mov r14, rdx
    ; mov rcx, rsi);

    code.iter().for_each(|&ir| {
        use BfIR::*;
        match ir {
            AddPtr(x) => dynasm!(ops
                ; add rcx, x as i32
                ; jc -> overflow
                ; cmp rcx, r14
                ; jnb -> overflow
            ),
            SubPtr(x) => dynasm!(ops
                ; sub rcx, x as i32
                ; jc  -> overflow
                ; cmp rcx, r13
                ; jb  -> overflow
            ),
            AddVal(x) => dynasm!(ops
                ; add BYTE [rcx], x as i8
            ),
            SubVal(x) => dynasm!(ops
                ; sub BYTE [rcx], x as i8
            ),
            GetByte => dynasm!(ops
                ; mov  r15, rcx
                ; mov  rdi, r12
                ; mov  rsi, rcx
                ; mov  rax, QWORD BfVM::getbyte as _
                ; call rax
                ; test rax, rax
                ; jnz  ->io_error
                ; mov  rcx, r15
            ),
            PutByte => dynasm!(ops
                ; mov  r15, rcx
                ; mov  rdi, r12
                ; mov  rsi, rcx
                ; mov  rax, QWORD BfVM::putbyte as _
                ; call rax
                ; test rax, rax
                ; jnz  -> io_error
                ; mov  rcx, r15
            ),
            Jz => {
                let left = ops.new_dynamic_label();
                let right = ops.new_dynamic_label();
                loops.push((left, right));
                dynasm!(ops
                    ; cmp BYTE [rcx], 0
                    ; jz => right
                    ; => left
                )
            }
            Jnz => {
                let (left, right) = loops.pop().expect("Not enough loops");
                dynasm!(ops
                    ; cmp BYTE [rcx], 0
                    ; jnz => left
                    ; => right
                )
            }
        };
    });

    dynasm!(ops
        ; xor rax, rax
        ; jmp > exit
        ; -> overflow:
        ; mov rax, QWORD BfVM::overflow_error as _
        ; call rax
        ; jmp >exit
        ; -> io_error:
        ; exit:
        ; pop rdx
        ; ret
    );

    let code = ops.finalize().expect("Failed to finalize code");

    Ok((code, start))
}
