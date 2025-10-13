export const JSON_RPC_VERSION = "2.0" as const;

export type JsonRpcId = number | string | null;

interface JsonRpcEnvelope {
  jsonrpc: typeof JSON_RPC_VERSION;
}

export interface JsonRpcRequest<P = unknown> extends JsonRpcEnvelope {
  id: JsonRpcId;
  method: string;
  params?: P;
}

export interface JsonRpcNotification<P = unknown> extends JsonRpcEnvelope {
  method: string;
  params?: P;
}

export interface JsonRpcSuccess<R = unknown> extends JsonRpcEnvelope {
  id: JsonRpcId;
  result: R;
}

export interface JsonRpcError<E = unknown> extends JsonRpcEnvelope {
  id: JsonRpcId | null;
  error: JsonRpcErrorObject<E>;
}

export interface JsonRpcErrorObject<E = unknown> {
  code: number;
  message: string;
  data?: E;
}

export type JsonRpcMessage =
  | JsonRpcRequest
  | JsonRpcNotification
  | JsonRpcSuccess
  | JsonRpcError;

export const JSON_RPC_ERROR_CODES = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
} as const;

const hasProperty = <K extends PropertyKey>(value: unknown, key: K): value is Record<K, unknown> =>
  typeof value === "object" && value !== null && key in value;

export class JsonRpcException<E = unknown> extends Error {
  public readonly code: number;
  public readonly data?: E;

  constructor({ code, message, data }: JsonRpcErrorObject<E>) {
    super(message);
    this.code = code;
    this.data = data;
    this.name = "JsonRpcException";
  }
}

export const isJsonRpcRequest = (message: JsonRpcMessage): message is JsonRpcRequest => {
  return hasProperty(message, "method") && hasProperty(message, "id");
};

export const isJsonRpcNotification = (message: JsonRpcMessage): message is JsonRpcNotification => {
  return hasProperty(message, "method") && !hasProperty(message, "id");
};

export const isJsonRpcSuccess = (message: JsonRpcMessage): message is JsonRpcSuccess => {
  return hasProperty(message, "result");
};

export const isJsonRpcError = (message: JsonRpcMessage): message is JsonRpcError => {
  return hasProperty(message, "error");
};

export type RpcMethodSpec<Params = unknown, Result = unknown, Error = unknown> = {
  readonly params: Params;
  readonly result: Result;
  readonly error?: Error;
};

export type RpcSchema = Record<string, RpcMethodSpec>;

export type RpcParams<S extends RpcSchema, M extends keyof S> = S[M]["params"];
export type RpcResult<S extends RpcSchema, M extends keyof S> = S[M]["result"];
export type RpcError<S extends RpcSchema, M extends keyof S> = S[M]["error"];
