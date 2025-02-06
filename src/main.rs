use anyhow::{anyhow, Result};
use std::{env, fs, path::Path, process};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
mod impls;
mod util;

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

    // Create the Unix listener
    let listener = UnixListener::bind(socket_path)?;

    // Accept incoming connections
    loop {
        let (stream, _addr) = listener.accept().await?;

        // Spawn a task to handle the connection
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                eprintln!("Error handling client: {}", e);
            }
        });
    }
}

async fn handle_client(mut stream: UnixStream) -> Result<()> {
    let _request = read_request(&mut stream).await?;
    Ok(())
    /*
    let mut buf = [0; 1024];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break; // Connection closed
        }

        let received = String::from_utf8_lossy(&buf[..n]);
        print!("Received: {}", received);

        // Echo back the received message
        stream.write_all(&buf[..n]).await?;
    }

    println!("Client disconnected");
    Ok(())
    */
}

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

fn err_response(id: Option<String>, err: anyhow::Error) -> Response {
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

async fn read_request(stream: &mut UnixStream) -> Result<Request> {
    let mut buffer = [0; 1024 * 2];
    let mut data = Vec::new();
    let req: Option<Request>;

    loop {
        let bytes_read = stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            req = Some(util::decode_request(&data)?);
            break; // End of stream reached
        }

        // Append the read data
        data.extend_from_slice(&buffer[..bytes_read]);

        match util::decode_request(&data) {
            Ok(r) => {
                req = Some(r);
                break;
            }
            Err(_e) => continue,
        }
    }

    req.ok_or(anyhow!("request is None"))
}
