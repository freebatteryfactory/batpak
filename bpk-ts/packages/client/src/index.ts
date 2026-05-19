/**
 * NETBAT/1 frame client.
 *
 * Phase 0: TCP only. The line protocol is the bytes documented in
 * `bpk-lib/crates/netbat/src/transport.rs:404-454`:
 *
 *     NETBAT/1 CALL <operation-name> <hex-input>\n
 *     OK <hex-output>\n
 *     ERR <code> <hex-message>\n
 *
 * - Hex is lowercase on encode; both cases accepted on decode.
 * - ERR `<code>` is a stable ASCII token from
 *   `NetbatError::code()`. The message half is hex of plain UTF-8 text
 *   (NOT MessagePack — do not pass it through `@batpak/canonical`'s
 *   `decode`).
 * - Operation-name grammar: ASCII graphic
 *   `[A-Za-z0-9._-]`, cannot start/end with `.`, cannot contain `..`,
 *   length <= 128 bytes.
 *
 * Byte bounds match the netbat defaults:
 *   line  <= 64 KiB
 *   input <= 32 KiB
 *   output<= 32 KiB
 */

import { decodeHex, encodeHex } from "@batpak/canonical";

export const NETBAT_VERSION = "NETBAT/1";
export const CALL_VERB = "CALL";

export const DEFAULT_MAX_LINE_BYTES = 64 * 1024;
export const DEFAULT_MAX_INPUT_BYTES = 32 * 1024;
export const DEFAULT_MAX_OUTPUT_BYTES = 32 * 1024;
export const MAX_OPERATION_NAME_BYTES = 128;

/**
 * Branded TS counterpart of Rust's `syncbat::OperationName` newtype: a
 * string that has been validated against the netbat operation-name
 * grammar. Construct only via {@link validateOperationName}; downstream
 * code should accept this type instead of re-parsing the grammar.
 *
 * The brand is structural — every {@link OperationName} is assignable to
 * a plain `string`, but a plain `string` is not assignable to
 * {@link OperationName} without going through the validator.
 */
export type OperationName = string & { readonly __brand: "OperationName" };

const OPERATION_NAME_PATTERN = /^[A-Za-z0-9._-]+$/u;

/** All NETBAT/1 error codes emitted by `netbat::NetbatError::code()`. */
export const NETBAT_ERROR_CODES = [
  "io",
  "empty_stream",
  "line_too_long",
  "malformed_request",
  "unsupported_protocol_version",
  "operation_name_too_long",
  "input_too_large",
  "output_too_large",
  "unknown_operation",
  "missing_handler",
  "handler",
  "receipt_sink",
] as const;
export type NetbatErrorCode = (typeof NETBAT_ERROR_CODES)[number];

export interface NetbatError {
  readonly kind: "netbat-error";
  readonly code: NetbatErrorCode;
  /** UTF-8 text decoded from the `<hex-message>` half of the ERR frame. */
  readonly message: string;
}

export interface NetbatOk {
  readonly kind: "netbat-ok";
  readonly output: Uint8Array;
}

export type NetbatResponse = NetbatOk | NetbatError;

export interface RequestFrame {
  /**
   * Validated operation name. `OperationName` is structurally a string, so
   * the field is read-compatible with any existing consumer that expects a
   * plain `string`. New code should keep names branded by funnelling them
   * through {@link validateOperationName}.
   */
  readonly operation: OperationName;
  readonly input: Uint8Array;
}

export class FrameValidationError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "FrameValidationError";
    this.code = code;
  }
}

/**
 * Validate an operation name against the netbat grammar and brand it as an
 * {@link OperationName}. Throws on empty, too-long, illegal characters,
 * leading/trailing `.`, or `..` substrings.
 *
 * This is the TS counterpart of the substrate-wide
 * `syncbat::OperationName::new` validating constructor. Every layer
 * (encode, parse, dispatch) should funnel through this function so the
 * grammar lives in exactly one place.
 */
export function validateOperationName(operation: string): OperationName {
  if (operation.length === 0) {
    throw new FrameValidationError("malformed_request", "operation name is empty");
  }
  const utf8Length = new TextEncoder().encode(operation).length;
  if (utf8Length > MAX_OPERATION_NAME_BYTES) {
    throw new FrameValidationError(
      "operation_name_too_long",
      `operation name ${utf8Length} bytes exceeds ${MAX_OPERATION_NAME_BYTES}`,
    );
  }
  if (!OPERATION_NAME_PATTERN.test(operation)) {
    throw new FrameValidationError(
      "malformed_request",
      `operation name ${JSON.stringify(operation)} contains illegal characters (allowed: [A-Za-z0-9._-])`,
    );
  }
  if (operation.startsWith(".") || operation.endsWith(".")) {
    throw new FrameValidationError(
      "malformed_request",
      `operation name ${JSON.stringify(operation)} cannot start or end with '.'`,
    );
  }
  if (operation.includes("..")) {
    throw new FrameValidationError(
      "malformed_request",
      `operation name ${JSON.stringify(operation)} cannot contain '..'`,
    );
  }
  return operation as OperationName;
}

/**
 * Encode a CALL request frame, including the trailing `\n`.
 *
 * Accepts either a plain `string` (which is validated and brand-promoted
 * internally) or an already-branded {@link OperationName}. Either way the
 * frame is only emitted when the name passes the netbat grammar.
 */
export function encodeRequest(operation: string | OperationName, input: Uint8Array): Uint8Array {
  const validated = validateOperationName(operation);
  if (input.length > DEFAULT_MAX_INPUT_BYTES) {
    throw new FrameValidationError(
      "input_too_large",
      `input ${input.length} bytes exceeds ${DEFAULT_MAX_INPUT_BYTES}`,
    );
  }
  const prefix = `${NETBAT_VERSION} ${CALL_VERB} ${validated} `;
  const prefixBytes = new TextEncoder().encode(prefix);
  const hex = encodeHex(input);
  const hexBytes = new TextEncoder().encode(hex);
  const out = new Uint8Array(prefixBytes.length + hexBytes.length + 1);
  out.set(prefixBytes, 0);
  out.set(hexBytes, prefixBytes.length);
  out[out.length - 1] = 0x0a;
  return out;
}

/** Parse a CALL request frame (including or excluding the trailing newline). */
export function parseRequestFrame(line: Uint8Array): RequestFrame {
  const text = trimNewline(new TextDecoder("utf-8", { fatal: true }).decode(line));
  const prefix = `${NETBAT_VERSION} ${CALL_VERB} `;
  if (!text.startsWith(prefix)) {
    throw new FrameValidationError(
      "malformed_request",
      `request frame must start with ${JSON.stringify(prefix)}`,
    );
  }
  const remainder = text.slice(prefix.length);
  const spaceIdx = remainder.indexOf(" ");
  if (spaceIdx < 0) {
    throw new FrameValidationError(
      "malformed_request",
      "request frame missing space between operation and hex payload",
    );
  }
  const operation = validateOperationName(remainder.slice(0, spaceIdx));
  const hex = remainder.slice(spaceIdx + 1);
  const input = decodeHex(hex);
  return { operation, input };
}

/** Parse an OK or ERR response frame (including or excluding the trailing newline). */
export function parseResponseFrame(line: Uint8Array): NetbatResponse {
  const text = trimNewline(new TextDecoder("utf-8", { fatal: true }).decode(line));
  if (text.startsWith("OK ")) {
    const hex = text.slice(3);
    return { kind: "netbat-ok", output: decodeHex(hex) };
  }
  if (text.startsWith("ERR ")) {
    const remainder = text.slice(4);
    const spaceIdx = remainder.indexOf(" ");
    if (spaceIdx < 0) {
      throw new FrameValidationError(
        "malformed_request",
        "ERR frame missing space between code and hex message",
      );
    }
    const codeRaw = remainder.slice(0, spaceIdx);
    const hex = remainder.slice(spaceIdx + 1);
    if (!isNetbatErrorCode(codeRaw)) {
      throw new FrameValidationError(
        "malformed_request",
        `ERR frame carries unknown code ${JSON.stringify(codeRaw)} (expected one of ${NETBAT_ERROR_CODES.join(", ")})`,
      );
    }
    const messageBytes = decodeHex(hex);
    // The Rust side emits `error.to_string().as_bytes()` — plain UTF-8,
    // never MessagePack. Decode as UTF-8 only.
    const message = new TextDecoder("utf-8", { fatal: true }).decode(messageBytes);
    return { kind: "netbat-error", code: codeRaw, message };
  }
  throw new FrameValidationError(
    "malformed_request",
    `response frame must start with "OK " or "ERR " (got ${JSON.stringify(text.slice(0, 8))})`,
  );
}

function isNetbatErrorCode(value: string): value is NetbatErrorCode {
  for (const code of NETBAT_ERROR_CODES) {
    if (code === value) return true;
  }
  return false;
}

function trimNewline(text: string): string {
  if (text.endsWith("\r\n")) return text.slice(0, -2);
  if (text.endsWith("\n")) return text.slice(0, -1);
  return text;
}

/**
 * Read a single line from a Node `net.Socket`-like readable. The line
 * includes the trailing `\n` byte. Refuses lines longer than
 * `DEFAULT_MAX_LINE_BYTES`.
 */
export async function readLine(
  socket: NodeReadable,
  maxBytes: number = DEFAULT_MAX_LINE_BYTES,
): Promise<Uint8Array> {
  const buffered: number[] = [];
  return await new Promise<Uint8Array>((resolve, reject) => {
    const onData = (chunk: Buffer | Uint8Array) => {
      const bytes = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
      for (const byte of bytes) {
        buffered.push(byte);
        if (buffered.length > maxBytes) {
          cleanup();
          reject(new FrameValidationError("line_too_long", `line exceeded ${maxBytes} bytes`));
          return;
        }
        if (byte === 0x0a) {
          cleanup();
          resolve(new Uint8Array(buffered));
          return;
        }
      }
    };
    const onEnd = () => {
      cleanup();
      if (buffered.length === 0) {
        reject(new FrameValidationError("empty_stream", "stream closed before any bytes"));
      } else {
        // Tolerate trailing line missing newline.
        resolve(new Uint8Array(buffered));
      }
    };
    const onError = (error: Error) => {
      cleanup();
      reject(error);
    };
    const cleanup = () => {
      socket.off("data", onData);
      socket.off("end", onEnd);
      socket.off("error", onError);
    };
    socket.on("data", onData);
    socket.once("end", onEnd);
    socket.once("error", onError);
  });
}

/** Minimal duck-typed Node readable used by {@link readLine}. */
export interface NodeReadable {
  on(event: "data", listener: (chunk: Buffer | Uint8Array) => void): unknown;
  once(event: "end", listener: () => void): unknown;
  once(event: "error", listener: (error: Error) => void): unknown;
  // `off` matches Node's EventEmitter.off shape. We never call it with
  // anything other than listeners we previously registered via on/once
  // above — using `unknown[]` instead of `any[]` keeps the typed-lint
  // bundle happy while still accepting a Socket structurally.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- rationale: matches Node EventEmitter.off ABI; we never invoke it
  off(eventName: string | symbol, listener: (...args: any[]) => void): unknown;
}

/**
 * Issue a single CALL/response roundtrip over a Node `net.Socket`.
 *
 * The socket is consumed for this call (one request, one response).
 */
export async function call(
  socket: NodeSocketLike,
  operation: string | OperationName,
  input: Uint8Array,
): Promise<NetbatResponse> {
  const frame = encodeRequest(operation, input);
  await new Promise<void>((resolve, reject) => {
    socket.write(frame, (error) => (error ? reject(error) : resolve()));
  });
  const line = await readLine(socket);
  return parseResponseFrame(line);
}

/** Minimal Node `net.Socket`-shaped writer/reader used by {@link call}. */
export interface NodeSocketLike extends NodeReadable {
  write(data: Uint8Array, callback?: (error: Error | null | undefined) => void): boolean;
}
