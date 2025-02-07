use anyhow::{anyhow, Result};
use bendy::decoding::FromBencode;
use bendy::encoding::ToBencode;
use std::future::Future;
use std::pin::Pin;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
mod impls;

#[derive(Debug, PartialEq)]
pub enum Op {
    Describe,
    Invoke,
}

impl Op {
    fn from_str(s: &str) -> Result<Op, String> {
        match s {
            "describe" => Ok(Op::Describe),
            "invoke" => Ok(Op::Invoke),
            _ => Err(format!("Invalid operation: {}", s)),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Request {
    op: Op,
    id: Option<String>,
    var: Option<String>,
    args: Option<String>,
}

#[derive(PartialEq, Debug)]
pub struct Var {
    name: String,
}

#[derive(PartialEq, Debug)]
pub struct Namespace {
    name: String,
    vars: Vec<Var>,
}

#[derive(PartialEq, Debug)]
pub struct DescribeResponse {
    format: String,
    namespaces: Vec<Namespace>,
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Done,
    Error,
}

impl Status {
    fn as_str(&self) -> &str {
        match self {
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ErrorResponse {
    id: Option<String>,
    status: Status,
    ex_message: String,
    //ex_data: Option<String>,
}

pub fn err_response(id: Option<String>, err: anyhow::Error) -> Response {
    Response::Error(ErrorResponse {
        id,
        status: Status::Error,
        ex_message: err.to_string(),
    })
}

#[derive(PartialEq, Debug)]
pub struct InvokeResponse {
    id: String,
    status: Status,
    value: Vec<u8>,
}

#[derive(Debug)]
pub enum Response {
    Describe(DescribeResponse),
    Invoke(InvokeResponse),
    Error(ErrorResponse),
}

pub async fn read_request(stream: &mut UnixStream) -> Result<Request> {
    let mut buffer = [0; 1024 * 2];
    let mut data = Vec::new();
    let req: Option<Request>;

    loop {
        let bytes_read = stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            req = Some(decode_request(&data)?);
            break; // End of stream reached
        }

        // Append the read data
        data.extend_from_slice(&buffer[..bytes_read]);

        match decode_request(&data) {
            Ok(r) => {
                req = Some(r);
                break;
            }
            Err(_e) => continue,
        }
    }

    req.ok_or(anyhow!("request is None"))
}

pub fn decode_request(buffer: &[u8]) -> Result<Request> {
    // Check if the last byte is `e` (ASCII value for 'e') which marks dictionary termination
    if buffer[buffer.len() - 1] == b'e' {
        Request::from_bencode(buffer).map_err(|e| anyhow!("{}", e))
    } else {
        Err(anyhow!("keep reading"))
    }
}

pub type HandlerFn = fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>;

pub async fn run_server(socket_path: &str, handler: HandlerFn) -> Result<()> {
    // Create the Unix listener
    let listener = UnixListener::bind(socket_path)?;

    // Accept incoming connections
    loop {
        let (stream, _addr) = listener.accept().await?;

        // Spawn a task to handle the connection
        tokio::spawn(async move { handle_client(stream, handler).await });
    }
}

async fn handle_client(mut stream: UnixStream, handler: HandlerFn) {
    let request = read_request(&mut stream).await;

    match request {
        Ok(req) => {
            let response = handler(req).await;
            match response {
                Ok(response) => match response.to_bencode() {
                    Ok(buf) => {
                        if let Err(err) = stream.write_all(&buf).await {
                            eprintln!("writing out stream failed {}", err);
                        }
                    }
                    Err(err) => {
                        let er = err_response(None, anyhow::Error::msg(err.to_string()));
                        if let Ok(e_buf) = er.to_bencode() {
                            if let Err(err) = stream.write_all(&e_buf).await {
                                eprintln!("failed writing out err stream {}", err);
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("handle_request failed with `{}`", e);
                    let er = err_response(None, e);
                    match er.to_bencode() {
                        Ok(e_buf) => {
                            if let Err(err) = stream.write_all(&e_buf).await {
                                eprintln!("failed writing out stream {}", err);
                            }
                        }
                        Err(err) => {
                            eprintln!("trouble encoding error response {}", err);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("trouble reading request from the stream {}", e);
        }
    }
}
