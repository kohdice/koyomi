use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so piping to `head` etc. exits cleanly
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    match koyomi::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("koyomi: {e:#}");
            ExitCode::FAILURE
        }
    }
}
