use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use runner_core_v2::proto::{Envelope, Outbound, Request, Response};

pub type FramedStream = Framed<tokio::net::UnixStream, LengthDelimitedCodec>;

pub fn framed(stream: tokio::net::UnixStream) -> FramedStream {
    Framed::new(stream, LengthDelimitedCodec::new())
}

pub async fn send_request(framed: &mut FramedStream, req: &Envelope<Request>) -> io::Result<()> {
    let bytes =
        serde_json::to_vec(req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    framed.send(bytes.into()).await
}

pub async fn read_request(framed: &mut FramedStream) -> io::Result<Option<Envelope<Request>>> {
    let frame = match framed.next().await {
        None => return Ok(None),
        Some(r) => r?,
    };

    let req = serde_json::from_slice::<Envelope<Request>>(&frame)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(req))
}

pub async fn read_response(framed: &mut FramedStream) -> io::Result<Envelope<Response>> {
    let frame = framed
        .next()
        .await
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "socket closed"))??;

    serde_json::from_slice::<Envelope<Response>>(&frame)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub async fn send_outbound(framed: &mut FramedStream, msg: &Outbound) -> io::Result<()> {
    let bytes =
        serde_json::to_vec(msg).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    framed.send(bytes.into()).await
}

pub async fn read_outbound(framed: &mut FramedStream) -> io::Result<Option<Outbound>> {
    let frame = match framed.next().await {
        None => return Ok(None),
        Some(r) => r?,
    };

    let msg = serde_json::from_slice::<Outbound>(&frame)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(msg))
}
