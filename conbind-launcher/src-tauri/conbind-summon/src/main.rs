use std::process::ExitCode;

fn main() -> ExitCode {
    match conbind_summon::run(std::env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("conbind-summon: {error}");
            ExitCode::FAILURE
        }
    }
}
