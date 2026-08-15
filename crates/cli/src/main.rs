use std::process::ExitCode;

fn main() -> ExitCode {
    match mbx::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mbx: {error}");
            ExitCode::from(2)
        }
    }
}
