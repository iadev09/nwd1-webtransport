// ! nwd1-webtransport
//!
//! Part of the **nwd1 family** — *Network Wire Data v1* transport suite:
//! - [`netid64`] → unique 64‑bit identifiers for network‑scoped entities
//! - [`nwd1`] → core binary frame definition
//! - [`nwd1-quic`] → QUIC transport adapter
//! - [`nwd1-webtransport`] → WebTransport (HTTP/3) adapter for browsers
//!
//! Together they form a unified, transport‑safe data layer from raw QUIC to browser.
//!
//! "nwd1" = **Network Wire Data v1** — a minimal binary frame: `MAGIC | LEN | ID | KIND | VER | PAYLOAD`.
//! This crate provides generic async helpers that work over any Tokio `AsyncRead/AsyncWrite`
//!
//! Design goals:
//! - Transport-agnostic helpers (server <-> browser via WebTransport planned)
//! - Zero ambiguity in framing, big-endian network order
//! - Early MAGIC check and length caps for safety
//!

use bytes::BytesMut;
use nwd1::{decode, encode, Frame, MAGIC};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Hard safety ceiling against pathological allocations (8 MiB)
pub const MAX_FRAME_LEN_HARD: usize = 8 * 1024 * 1024;
/// Reasonable default soft cap for browser-oriented traffic (256 KiB)
pub const DEFAULT_FRAME_LEN_SOFT: usize = 256 * 1024;
const HEADER_LEN: usize = 8; // MAGIC(4) + LEN(4)

/// Send a single nwd1 frame over any Tokio `AsyncWrite`.
pub async fn send_frame<W: AsyncWrite + Unpin>(writer: &mut W, frame: &Frame) -> io::Result<()> {
    let data = encode(frame);
    writer.write_all(&data).await?;
    Ok(())
}

/// Receive a single nwd1 frame from any Tokio `AsyncRead`.
/// Returns `Ok(None)` on graceful EOF before a full header is read.
pub async fn recv_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    soft_cap: usize,
) -> io::Result<Option<Frame>> {
    // Read fixed 8-byte header
    let mut header = [0u8; HEADER_LEN];
    if let Err(e) = reader.read_exact(&mut header).await {
        return if e.kind() == io::ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(e)
        };
    }

    // Fast-fail on bad magic
    if &header[..4] != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "nwd1 bad magic"));
    }

    // Parse LEN (BE u32)
    let len = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;

    // Enforce caps: soft (configurable) and hard (absolute)
    if len > soft_cap || len > MAX_FRAME_LEN_HARD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "nwd1 frame too large",
        ));
    }

    // Read body
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;

    // Compose into a contiguous buffer for `nwd1::decode`
    let mut buf = BytesMut::with_capacity(HEADER_LEN + len);
    buf.extend_from_slice(&header);
    buf.extend_from_slice(&body);

    match decode(&buf.freeze()) {
        Ok(frame) => Ok(Some(frame)),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "nwd1 decode error",
        )),
    }
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use netid64::NetId64;
	// dev-dependency

    #[tokio::test]
    async fn roundtrip_via_inmemory() {
        // Build a frame
        let frame = Frame {
            id: NetId64::make(1, 7, 42),
            kind: 1,
            ver: 1,
            payload: Bytes::from_static(b"hello"),
        };

        // Encode and pipe through an in-memory cursor to simulate IO
        let data = encode(&frame);
        let cursor = tokio::io::duplex(64 * 1024);
        let (mut w, mut r) = cursor;

        // Writer task
        let written = data.clone();
        let send = async move { w.write_all(&written).await };

        // Reader task
        let recv = async move { recv_frame(&mut r, DEFAULT_FRAME_LEN_SOFT).await };

        let (sw, rr) = tokio::join!(send, recv);
        sw.expect("write ok");
        let decoded = rr.expect("io ok").expect("some");

        assert_eq!(decoded.id.raw(), frame.id.raw());
        assert_eq!(decoded.kind, frame.kind);
        assert_eq!(decoded.ver, frame.ver);
        assert_eq!(decoded.payload, frame.payload);
    }
}
