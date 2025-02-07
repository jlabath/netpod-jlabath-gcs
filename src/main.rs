use anyhow::{anyhow, Result};
use std::{env, fs, future::Future, path::Path, pin::Pin, process};
mod netpod;
use netpod::{err_response, run_server, Request, Response};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Expected exactly one argument!");
        process::exit(1);
    }

    let socket_path = &args[1];

    // Remove existing socket file if it exists
    if Path::new(socket_path).exists() {
        fs::remove_file(socket_path)?;
    }

    run_server(socket_path, handler_boxed).await?;
    Ok(())
}

async fn handler(_request: Request) -> Result<Response> {
    eprintln!("died as expected");
    Ok(err_response(None, anyhow!("bad")))
}

fn handler_boxed(req: Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
    Box::pin(handler(req))
}
