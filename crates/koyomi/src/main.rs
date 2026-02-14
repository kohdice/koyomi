use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match koyomi::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
