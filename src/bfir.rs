use crate::errors::{CompileError, CompileErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BfIR {
    AddVal(u8),
    SubVal(u8),
    AddPtr(u8),
    SubPtr(u8),
    GetByte,
    PutByte,
    Jz,
    Jnz,
}

pub fn compile(src: &str) -> Result<Vec<BfIR>, CompileError> {
    let mut code = vec![];
    let mut stk = vec![];

    let mut line = 1;
    let mut col = 1;

    for ch in src.chars() {
        col += 1;
        match ch {
            '\n' => {
                line += 1;
                col = 0;
            }

            '+' => code.push(BfIR::AddVal(1)),
            '-' => code.push(BfIR::SubVal(1)),
            '>' => code.push(BfIR::AddPtr(1)),
            '<' => code.push(BfIR::SubPtr(1)),
            ',' => code.push(BfIR::GetByte),
            '.' => code.push(BfIR::PutByte),
            '[' => {
                let pos = code.len();
                stk.push((pos, line, col));
                code.push(BfIR::Jz);
            }
            ']' => {
                stk.pop().ok_or(CompileError {
                    line,
                    col,
                    kind: CompileErrorKind::UnexpectedRightBracket,
                })?;
                code.push(BfIR::Jnz);
            }
            _ => {}
        }
    }

    if let Some((_, line, col)) = stk.pop() {
        return Err(CompileError {
            line,
            col,
            kind: CompileErrorKind::UnclosedLeftBracket,
        });
    }

    Ok(code)
}

pub fn optimize(code: &mut Vec<BfIR>) {
    let mut i = 0;
    let mut pc = 0;

    macro_rules! fold_ir {
        ($variant:ident, $x:ident) => {{
            let mut j = i + 1;
            while j < code.len() {
                if let $variant(d) = code[j] {
                    $x = $x.wrapping_add(d);
                } else {
                    break;
                }
                j += 1;
            }

            i = j;
            code[pc] = $variant($x);
            pc += 1;
        }};
    }

    macro_rules! normal_ir {
        () => {{
            code[pc] = code[i];
            pc += 1;
            i += 1;
        }};
    }

    use BfIR::*;
    while i < code.len() {
        match code[i] {
            AddVal(mut x) => fold_ir!(AddVal, x),
            SubVal(mut x) => fold_ir!(SubVal, x),
            AddPtr(mut x) => fold_ir!(AddPtr, x),
            SubPtr(mut x) => fold_ir!(SubPtr, x),
            GetByte => normal_ir!(),
            PutByte => normal_ir!(),
            Jz => normal_ir!(),
            Jnz => normal_ir!(),
        }
    }

    code.truncate(pc);
    code.shrink_to_fit();
}

#[test]
fn test_compile() {
    assert_eq!(
        compile("+[,.]").unwrap(),
        vec![
            BfIR::AddVal(1),
            BfIR::Jz,
            BfIR::GetByte,
            BfIR::PutByte,
            BfIR::Jnz,
        ]
    );

    match compile("[").unwrap_err().kind {
        CompileErrorKind::UnclosedLeftBracket => {}
        _ => panic!(),
    };

    match compile("]").unwrap_err().kind {
        CompileErrorKind::UnexpectedRightBracket => {}
        _ => panic!(),
    };

    let mut code = compile("[+++++]").unwrap();
    optimize(&mut code);
    assert_eq!(code, vec![BfIR::Jz, BfIR::AddVal(5), BfIR::Jnz]);
}
