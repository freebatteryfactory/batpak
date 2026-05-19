/**
 * Recursive canonical MessagePack decoder.
 *
 * Walks a byte buffer via {@link Reader} and reconstructs the
 * JSON-shaped value. Mirrors the encode side in `./encode.ts` for the
 * Phase 0 subset (see `./index.ts` for the subset contract).
 */

import { CanonicalDecodeError, Reader } from "./reader.js";

const POS_FIXINT_MAX = 0x7f;

const NIL = 0xc0;
const FALSE = 0xc2;
const TRUE = 0xc3;
const UINT8 = 0xcc;
const UINT16 = 0xcd;
const UINT32 = 0xce;
const UINT64 = 0xcf;
const STR8 = 0xd9;
const STR16 = 0xda;
const STR32 = 0xdb;
const ARRAY16 = 0xdc;
const ARRAY32 = 0xdd;
const MAP16 = 0xde;
const MAP32 = 0xdf;

/**
 * Decode canonical named-field MessagePack bytes back into a
 * JSON-shaped value.
 */
export function decode(bytes: Uint8Array): unknown {
  const reader = new Reader(bytes);
  const value = decodeValue(reader);
  if (!reader.atEnd()) {
    throw new CanonicalDecodeError(
      "trailing_bytes",
      `decoder finished at offset ${reader.offset} but buffer has ${bytes.length} bytes`,
    );
  }
  return value;
}

function decodeValue(reader: Reader): unknown {
  const head = reader.readByte();
  // Positive fixint
  if (head <= POS_FIXINT_MAX) {
    return head;
  }
  // fixmap
  if (head >= 0x80 && head <= 0x8f) {
    return decodeMap(reader, head & 0x0f);
  }
  // fixarray
  if (head >= 0x90 && head <= 0x9f) {
    return decodeArray(reader, head & 0x0f);
  }
  // fixstr
  if (head >= 0xa0 && head <= 0xbf) {
    return decodeString(reader, head & 0x1f);
  }
  switch (head) {
    case NIL:
      return null;
    case FALSE:
      return false;
    case TRUE:
      return true;
    case UINT8:
      return reader.readByte();
    case UINT16:
      return reader.readUInt16BE();
    case UINT32:
      return reader.readUInt32BE();
    case UINT64:
      return reader.readUInt64BE();
    case STR8:
      return decodeString(reader, reader.readByte());
    case STR16:
      return decodeString(reader, reader.readUInt16BE());
    case STR32:
      return decodeString(reader, reader.readUInt32BE());
    case ARRAY16:
      return decodeArray(reader, reader.readUInt16BE());
    case ARRAY32:
      return decodeArray(reader, reader.readUInt32BE());
    case MAP16:
      return decodeMap(reader, reader.readUInt16BE());
    case MAP32:
      return decodeMap(reader, reader.readUInt32BE());
    default:
      throw new CanonicalDecodeError(
        "unsupported_token",
        `Phase 0 canonical decoder does not handle MessagePack token 0x${head.toString(16).padStart(2, "0")}`,
      );
  }
}

function decodeString(reader: Reader, length: number): string {
  const bytes = reader.readBytes(length);
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

function decodeArray(reader: Reader, length: number): unknown[] {
  const out: unknown[] = [];
  for (let i = 0; i < length; i += 1) {
    out.push(decodeValue(reader));
  }
  return out;
}

function decodeMap(reader: Reader, length: number): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (let i = 0; i < length; i += 1) {
    const key = decodeValue(reader);
    if (typeof key !== "string") {
      throw new CanonicalDecodeError(
        "non_string_key",
        `Phase 0 canonical decoder requires string map keys (saw ${typeof key})`,
      );
    }
    out[key] = decodeValue(reader);
  }
  return out;
}
