use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::get::GetObjectRequest;
use netpod_base::{
    error::{from_error, NetpodError},
    invoke_response, run_server, HandlerFn, HandlerFuture, Request, Response,
};
use std::{
    collections::HashMap, convert::TryFrom, env, error::Error, fs, path::Path, process,
    result::Result, sync::Arc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    let config = ClientConfig::default().with_auth().await?;
    let client = Arc::new(Client::new(config));

    let meta_handler = create_handler(client.clone(), |res, req| Box::pin(handle_meta(res, req)));

    //build handler map
    let mut handler_map: HashMap<String, HandlerFn> = HashMap::new();
    handler_map.insert("netpod.jlabath.gcs/meta".to_string(), meta_handler);

    run_server(socket_path, handler_map).await?;
    Ok(())
}

fn create_handler<F>(shared_resource: Arc<Client>, my_handler: F) -> HandlerFn
where
    F: Fn(Arc<Client>, Request) -> HandlerFuture + Send + Sync + 'static,
{
    Box::new(move |req: Request| Box::pin(my_handler(shared_resource.clone(), req)))
}

async fn handle_meta(shared_resource: Arc<Client>, req: Request) -> Result<Response, NetpodError> {
    let args_str = req
        .args
        .ok_or_else(|| NetpodError::from("args are missing"))?;
    let decoded_args: Vec<String> =
        serde_json::from_str(&args_str).map_err(|e| NetpodError::from(e.to_string()))?;
    let gs_path = decoded_args
        .first()
        .ok_or_else(|| NetpodError::Message("no gs:// filepath given".to_string()))?;
    let o_req = GcsObjectRequest::try_from(gs_path.as_str())?;
    let object = shared_resource
        .get_object(&o_req.0)
        .await
        .map_err(from_error)?;
    //eprintln!("Object: {:?}", &object);
    let data = serde_json::to_vec(&object).map_err(from_error)?;
    Ok(invoke_response(
        req.id.unwrap_or_else(|| "missing".to_string()),
        data,
    ))
}

/// Wrapper struct to allow `TryFrom<String>`
#[derive(Debug)]
struct GcsObjectRequest(GetObjectRequest);

impl TryFrom<&str> for GcsObjectRequest {
    type Error = NetpodError;

    fn try_from(gs_url: &str) -> Result<Self, Self::Error> {
        if !gs_url.starts_with("gs://") {
            return Err("does not start with gs://".into());
        }

        let path = &gs_url[5..]; // Remove "gs://"
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() < 2 {
            return Err(format!("expected bucket and object but got {}", path).into());
        }

        Ok(GcsObjectRequest(GetObjectRequest {
            bucket: parts[0].to_string(),
            object: parts[1].to_string(),
            ..Default::default()
        }))
    }
}
