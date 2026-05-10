use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

type Store = Arc<Mutex<HashMap<String, String>>>;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    let store: Store = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await?;
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, store).await {
                eprintln!("connection error: {e}");
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, store: Store) -> Result<()> {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        buf.clear();
        let n = socket.read_buf(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        let args = parse_resp(&buf[..n])?;
        let reply = dispatch(&args, &store).await;
        socket.write_all(&reply).await?;
    }
}

async fn dispatch(args: &[String], store: &Store) -> Vec<u8> {
    let Some(cmd) = args.first() else {
        return b"-ERR empty command\r\n".to_vec();
    };
    match cmd.to_ascii_uppercase().as_str() {
        "PING" => match args.get(1) {
            Some(msg) => bulk_string(msg),
            None => b"+PONG\r\n".to_vec(),
        },
        "ECHO" => match args.get(1) {
            Some(msg) => bulk_string(msg),
            None => b"-ERR wrong number of arguments for 'echo'\r\n".to_vec(),
        },
        "SET" => match (args.get(1), args.get(2)) {
            (Some(k), Some(v)) => {
                store.lock().await.insert(k.clone(), v.clone());
                b"+OK\r\n".to_vec()
            }
            _ => b"-ERR wrong number of arguments for 'set'\r\n".to_vec(),
        },
        "GET" => match args.get(1) {
            Some(k) => match store.lock().await.get(k) {
                Some(v) => bulk_string(v),
                None => b"$-1\r\n".to_vec(),
            },
            None => b"-ERR wrong number of arguments for 'get'\r\n".to_vec(),
        },
        other => format!("-ERR unknown command '{}'\r\n", other).into_bytes(),
    }
}

fn bulk_string(s: &str) -> Vec<u8> {
    format!("${}\r\n{}\r\n", s.len(), s).into_bytes()
}

fn parse_resp(buf: &[u8]) -> Result<Vec<String>> {
    let mut i = 0;
    if buf.first() != Some(&b'*') {
        return Err(anyhow!("expected RESP array"));
    }
    i += 1;
    let (count, used) = read_int_line(&buf[i..])?;
    i += used;

    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if buf.get(i) != Some(&b'$') {
            return Err(anyhow!("expected bulk string"));
        }
        i += 1;
        let (len, used) = read_int_line(&buf[i..])?;
        i += used;
        let end = i + len as usize;
        if end > buf.len() {
            return Err(anyhow!("bulk string truncated"));
        }
        out.push(String::from_utf8(buf[i..end].to_vec())?);
        i = end + 2;
    }
    Ok(out)
}

fn read_int_line(buf: &[u8]) -> Result<(i64, usize)> {
    let crlf = buf
        .windows(2)
        .position(|w| w == b"\r\n")
        .ok_or_else(|| anyhow!("missing CRLF"))?;
    let n = std::str::from_utf8(&buf[..crlf])?.parse::<i64>()?;
    Ok((n, crlf + 2))
}
