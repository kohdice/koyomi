use std::io::ErrorKind;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match koyomi::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if is_broken_pipe(&e) {
                return ExitCode::SUCCESS;
            }
            eprintln!("koyomi: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == ErrorKind::BrokenPipe)
    })
}
