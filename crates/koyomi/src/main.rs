#[tokio::main]
async fn main() {
    if let Err(e) = koyomi::run().await {
        let code = koyomi::exit_code_for(&e);
        eprintln!("koyomi: {e:#}");
        std::process::exit(code);
    }
}
