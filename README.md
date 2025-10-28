# 🜲 nwd1‑webtransport

**WebTransport (HTTP/3) adapter for the [nwd1](https://crates.io/crates/nwd1) binary frame protocol.**  
This crate bridges **server** and **browser** using QUIC and HTTP/3 streams.

---

## 🌐 What is `nwd1`?

`nwd1` stands for **Network Wire Data v1** — a minimal binary framing format designed for efficient, explicit, and
endian‑safe communication between peers.

**Frame layout:**

```
MAGIC (4B) | LEN (4B) | ID (8B) | KIND (1B) | VER (8B) | PAYLOAD (variable)
```

- `MAGIC`: Constant header `b"NWD1"`
- `LEN`: Big‑endian `u32` length of the rest of the frame
- `ID`: 64‑bit identifier ([NetId64](https://crates.io/crates/netid64))
- `KIND`: Message type / semantic code
- `VER`: Protocol version or schema version
- `PAYLOAD`: Raw data (binary or text)

`nwd1` defines the structure; this crate provides its transport over **WebTransport (HTTP/3)**.

---

## 🚀 Purpose of `nwd1‑webtransport`

This crate implements **transport‑safe** I/O helpers for `nwd1` frames over any Tokio `AsyncRead/AsyncWrite` stream and
is intended to be used with **WebTransport** connections to browsers.

### Core ideas

- 🧱 **Unified frame structure** — the same `nwd1::Frame` type works on QUIC, WebTransport, or other transports.
- 🧩 **Transport agnostic** — built atop `tokio`, `quinn`, and `h3`.
- 🧠 **Explicit semantics** — no hidden JSON encoding, no magic headers.
- ⚙️ **Safety** — strict MAGIC check, length caps (soft/hard), big‑endian order.

---

## 📦 Part of the *nwd1 family*

| Crate                                             | Purpose                                   |
|---------------------------------------------------|-------------------------------------------|
| [`netid64`](https://crates.io/crates/netid64)     | 64‑bit network‑scoped IDs                 |
| [`nwd1`](https://crates.io/crates/nwd1)           | Binary frame grammar                      |
| [`nwd1‑quic`](https://crates.io/crates/nwd1-quic) | QUIC transport for native apps            |
| **`nwd1‑webtransport`**                           | HTTP/3 / WebTransport bridge for browsers |

Together, they form a unified data stack from raw QUIC sockets to the browser DOM.

---

## ✳️ Example (simplified)

```rust
use nwd1::{Frame, encode, decode};
use nwd1_webtransport::{send_frame, recv_frame, DEFAULT_FRAME_LEN_SOFT};
use bytes::Bytes;
use netid64::NetId64;
use tokio::io::duplex;

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let frame = Frame {
		id: NetId64::make(1, 7, 42),
		kind: 1,
		ver: 1,
		payload: Bytes::from_static(b"hello"),
	};

	let (mut w, mut r) = duplex(64 * 1024);
	send_frame(&mut w, &frame).await?;
	if let Some(decoded) = recv_frame(&mut r, DEFAULT_FRAME_LEN_SOFT).await? {
		assert_eq!(decoded.payload, frame.payload);
	}
	Ok(())
}
```

---

## 🧭 Browser side (WebTransport)

In the browser, the same frame can be encoded/decoded using `Uint8Array`.  
For example:

```js
import {connectWT} from './nwd1-client.js';

const {frames} = await connectWT("https://your-domain/wt/nwd1");
for await (const f of frames()) {
    console.log("kind", f.kind, "payload", new TextDecoder().decode(f.payload));
}
```

See the [example client](https://github.com/iadev09/nwd1-webtransport/tree/master/demo) for details.

---

## 🔐 Safety & Limits

- **Hard cap:** 8 MiB per frame
- **Soft cap:** configurable (default 256 KiB)
- **MAGIC check:** invalid header immediately rejected
- **Endianness:** always big‑endian (network order)

---

## 🧱 Philosophy

`nwd1` seeks clarity over abstraction: one binary grammar, multiple carriers.  
Server and browser share the same semantic backbone — **a single frame of meaning in motion**.

---

## ⚖️ License

MIT OR Apache‑2.0