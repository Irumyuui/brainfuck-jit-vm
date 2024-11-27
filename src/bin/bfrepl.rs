use std::io::{BufRead, Write};

fn main() {
    println!("Brainfuck REPL.");

    'main_loop: loop {
        let mut stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();

        stdout.write(b"bf > ").unwrap();
        stdout.flush().unwrap();

        let mut code = String::new();
        'read: loop {
            let mut input = String::new();
            if let Err(e) = stdin.read_line(&mut input) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }

            let line = input.trim();
            if line.is_empty() {
                break 'read;
            }

            code.push_str(line);
        }

        let code = code.trim();
        if code.is_empty() {
            continue 'main_loop;
        }

        let eval_result = bf::bfvm::BfVM::new(&code, Box::new(stdin), Box::new(stdout), true)
            .and_then(|mut vm| vm.run());

        if let Err(e) = eval_result {
            eprintln!("Error: {}", e);
            continue 'main_loop;
        }
    }
}
