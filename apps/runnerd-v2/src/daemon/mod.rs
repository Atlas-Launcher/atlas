use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use runner_core_v2::PROTOCOL_VERSION;
use runner_core_v2::proto::{Envelope, Request, Response, RpcError, ErrorCode};

pub async fn serve(listener: UnixListener) -> std::io::Result<()> {
    loop {
        let (stream, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = handle_conn(stream).await;
        });
    }
}

async fn handle_conn(stream: tokio::net::UnixStream) -> std::io::Result<()> {
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    let frame = framed
        .next()
        .await
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "socket closed"))??;

    let req_env = serde_json::from_slice::<Envelope<Request>>(&frame)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let resp = match req_env.payload {
        Request::Ping { protocol_version, .. } => {
            if protocol_version != PROTOCOL_VERSION {
                Response::Error(RpcError {
                    code: ErrorCode::UnsupportedProtocol,
                    message: format!(
                        "protocol mismatch: client={protocol_version} daemon={PROTOCOL_VERSION}"
                    ),
                    details: Default::default(),
                })
            } else {
                Response::Pong {
                    daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                    protocol_version: PROTOCOL_VERSION,
                }
            }
        }
        _ => Response::Error(RpcError {
            code: ErrorCode::BadRequest,
            message: "unsupported request (not implemented yet)".into(),
            details: Default::default(),
        }),
    };

    let resp_env = Envelope { id: req_env.id, payload: resp };
    let out = serde_json::to_vec(&resp_env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    framed.send(out.into()).await?;
    Ok(())
}
