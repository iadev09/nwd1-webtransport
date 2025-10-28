// nwd1-client.js — browser-side frame codec for nwd1-webtransport
// All comments in English only.

const te = new TextEncoder();
export const MAGIC = te.encode("NWD1");
const HEADER_LEN = 8;

// BE helpers
const u32be = (n) => new Uint8Array([(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255]);
const beToU32 = (b) => (b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3];
const beToBig = (u8) => u8.reduce((a, b) => (a << 8n) + BigInt(b), 0n);
const bigTo8B = (x) => {
    let n = BigInt(x);
    const out = new Uint8Array(8);
    for (let i = 7; i >= 0; i--) {
        out[i] = Number(n & 0xFFn);
        n >>= 8n;
    }
    return out;
};

// Encode one nwd1 frame
export function encodeFrame({idBig, kind, verBig, payload}) {
    const id = bigTo8B(idBig);
    const ver = bigTo8B(verBig);
    const bodyLen = 8 + 1 + 8 + payload.length;
    const out = new Uint8Array(HEADER_LEN + bodyLen);
    out.set(MAGIC, 0);
    out.set(u32be(bodyLen), 4);
    out.set(id, 8);
    out[16] = kind & 0xFF;
    out.set(ver, 17);
    out.set(payload, 25);
    return out;
}

// Try decode from a buffer that might contain multiple frames
export function tryDecodeFrame(buf) {
    if (buf.length < HEADER_LEN) return {need: HEADER_LEN - buf.length};
    if (buf[0] !== 77 || buf[1] !== 87 || buf[2] !== 68 || buf[3] !== 49) throw new Error("bad magic");
    const len = beToU32(buf.slice(4, 8));
    const total = HEADER_LEN + len;
    if (buf.length < total) return {need: total - buf.length};

    const body = buf.slice(8, total);
    const idBig = beToBig(body.slice(0, 8));
    const kind = body[8];
    const verBig = beToBig(body.slice(9, 17));
    const payload = body.slice(17);
    return {frame: {idBig, kind, verBig, payload}, used: total};
}

// Connect to a WebTransport endpoint and return async frame stream
export async function connectWT(url) {
    const wt = new WebTransport(url);
    await wt.ready;

    const {readable, writable} = await wt.createBidirectionalStream();
    const writer = writable.getWriter();
    const reader = readable.getReader();
    let buf = new Uint8Array(0);

    async function send(frame) {
        await writer.write(encodeFrame(frame));
    }

    async function* frames() {
        while (true) {
            const {value, done} = await reader.read();
            if (done) return;
            if (!value) continue;

            const tmp = new Uint8Array(buf.length + value.length);
            tmp.set(buf, 0);
            tmp.set(value, buf.length);
            buf = tmp;

            while (buf.length >= HEADER_LEN) {
                const r = tryDecodeFrame(buf);
                if (!r.frame) break;
                yield r.frame;
                buf = buf.slice(r.used);
            }
        }
    }

    return {wt, send, frames};
}