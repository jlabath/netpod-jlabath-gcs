use anyhow::Result;
use std::{collections::HashMap, env, fs, path::Path, process, sync::Arc};
mod netpod;
use netpod::{invoke_response, run_server, HandlerFn, HandlerFuture, Request, Response};

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

    let shared_resource = Arc::new("conbtext".to_string());
    let meta_handler = create_handler(shared_resource, |res, req| Box::pin(handle_meta(res, req)));

    //build handler map
    let mut handler_map: HashMap<String, HandlerFn> = HashMap::new();
    handler_map.insert("netpod.jlabath.gcs/meta".to_string(), meta_handler);

    run_server(socket_path, handler_map).await?;
    Ok(())
}

fn create_handler<F>(shared_resource: Arc<String>, my_handler: F) -> HandlerFn
where
    F: Fn(Arc<String>, Request) -> HandlerFuture + Send + Sync + 'static,
{
    Box::new(move |req: Request| Box::pin(my_handler(shared_resource.clone(), req)))
}

async fn handle_meta(shared_resource: Arc<String>, req: Request) -> Result<Response> {
    eprintln!(
        "Hey I am in meta with shared_resurce: {}!",
        &shared_resource
    );
    Ok(invoke_response(
        req.id.unwrap_or("missing".to_string()),
        "{\"size\": 1000}".as_bytes().to_vec(),
    ))
}
