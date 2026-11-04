export type WireJson =
  | null
  | boolean
  | string
  | number
  | bigint
  | WireJson[]
  | { [key: string]: WireJson };

export class LosslessCodecError extends Error {
  override name = 'LosslessCodecError';
}

function hasLosslessRuntime(): boolean {
  if (typeof (JSON as JSON & { rawJSON?: (source: string) => unknown }).rawJSON !== 'function') return false;
  let sourceSeen = false;
  try {
    const parse = JSON.parse as unknown as (source: string, reviver?: (key: string, value: unknown, context?: { source: string }) => unknown) => unknown;
    parse('1', (_key: string, value: unknown, context?: { source: string }) => {
      sourceSeen = context?.source === '1';
      return value;
    });
  } catch {
    return false;
  }
  return sourceSeen;
}

export function assertLosslessRuntime(): void {
  if (!hasLosslessRuntime()) {
    throw new LosslessCodecError(
      'Lossless JSON codec requires native JSON.rawJSON and JSON.parse reviver context.source support.',
    );
  }
}

function encodeValue(value: WireJson): unknown {
  if (typeof value === 'bigint') {
    assertLosslessRuntime();
    const rawJSON = (JSON as JSON & { rawJSON: (source: string) => unknown }).rawJSON;
    return rawJSON(value.toString());
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new LosslessCodecError('JSON numbers must be finite.');
    }
    if (Number.isInteger(value) && !Number.isSafeInteger(value)) {
      throw new LosslessCodecError('Unsafe integer number would lose precision; use bigint instead.');
    }
    return value;
  }
  if (Array.isArray(value)) return value.map(encodeValue);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, encodeValue(entry)]));
  }
  return value;
}

export function encodeWire(value: WireJson): string {
  return JSON.stringify(encodeValue(value));
}

const integerToken = /^-?(?:0|[1-9]\d*)$/;

export function decodeWire(text: string): WireJson {
  assertLosslessRuntime();
  const parse = JSON.parse as (source: string, reviver?: (key: string, value: unknown, context?: { source: string }) => unknown) => WireJson;
  return parse(text, (_key, value, context) => {
    if (context && typeof context.source === 'string' && integerToken.test(context.source)) {
      return BigInt(context.source);
    }
    return value;
  });
}
