use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match koyomi::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>()
                && io_err.kind() == std::io::ErrorKind::BrokenPipe
            {
                return ExitCode::from(141);
            }
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
