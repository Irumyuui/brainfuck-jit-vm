use std::{
    io::{stdin, stdout},
    path::PathBuf,
};

use bf::bfvm::BfVM;
use clap::Parser;

#[derive(Debug, clap::Parser)]
#[clap(version)]
struct CliOpt {
    #[clap(name = "FILE")]
    file_path: PathBuf,

    #[clap(short = 'o', long = "optimize", help = "Enable optimize code")]
    optimize: bool,
}

fn main() {
    let opt = CliOpt::parse();

    let stdin = stdin();
    let stdout = stdout();

    let code = std::fs::read_to_string(&opt.file_path).expect("Failed to read file");

    let ret = BfVM::new(
        &code,
        Box::new(stdin.lock()),
        Box::new(stdout.lock()),
        opt.optimize,
    )
    .and_then(|mut vm| vm.run());

    if let Err(e) = &ret {
        eprintln!("bfjit: {}", e);
        std::process::exit(ret.is_err() as i32);
    }
}
