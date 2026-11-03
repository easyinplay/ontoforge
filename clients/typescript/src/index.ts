import createClient from 'openapi-fetch';
import type { components, paths } from './generated.js';
import { assertLosslessRuntime, decodeWire, encodeWire, type WireJson } from './codec.js';

export { assertLosslessRuntime, decodeWire, encodeWire, LosslessCodecError } from './codec.js';
export type { WireJson } from './codec.js';
export type { components, paths };

type Metadata = components['schemas']['Metadata'];
type ObjectRow = components['schemas']['ObjectRow'];
type QueryRequest = components['schemas']['QueryRequest'];
type ListPage = components['schemas']['ListPage'];
type AggregateRequest = components['schemas']['AggregateRequest'];
type AggregateResult = components['schemas']['AggregateResult'];
type ActionRequest = components['schemas']['ActionRequest'];
type ActionReceipt = components['schemas']['ActionReceipt'];
type HistoryPage = components['schemas']['HistoryPage'];
type HealthResponse = components['schemas']['HealthResponse'];
type ValidationResponse = components['schemas']['ValidationResponse'];

export interface OntoforgeClientOptions {
  baseUrl: string;
  bearer?: string;
  fetch?: typeof globalThis.fetch;
}

export class OntoforgeHttpError extends Error {
  readonly status: number;
  readonly apiName?: string;
  readonly code?: string;
  readonly requestId?: string;
  readonly body: unknown;

  constructor(response: Response, body: unknown) {
    const envelope = body && typeof body === 'object' ? body as Record<string, unknown> : undefined;
    super(
      typeof envelope?.code === 'string'
        ? `${envelope.code} (${response.status})`
        : `Ontoforge request failed with HTTP ${response.status}`,
    );
    this.name = 'OntoforgeHttpError';
    this.status = response.status;
    this.apiName = typeof envelope?.apiName === 'string' ? envelope.apiName : undefined;
    this.code = typeof envelope?.code === 'string' ? envelope.code : undefined;
    this.requestId = typeof envelope?.request_id === 'string' ? envelope.request_id : undefined;
    this.body = body;
  }
}

type CallOptions = { signal?: AbortSignal };

function responseObject(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid response object');
  return value as Record<string, unknown>;
}

function responseField(value: Record<string, unknown>, key: string, type: 'string' | 'bigint' | 'boolean'): void {
  if (typeof value[key] !== type) throw new Error(`Invalid response field ${key}: expected ${type}`);
  if (type === 'bigint' && ((value[key] as bigint) < -9223372036854775808n || (value[key] as bigint) > 9223372036854775807n)) {
    throw new Error(`Invalid response integer ${key}`);
  }
}

function responseArray(value: Record<string, unknown>, key: string): unknown[] {
  if (!Array.isArray(value[key])) throw new Error(`Invalid response array ${key}`);
  return value[key];
}

function responseOrdinal(value: Record<string, unknown>, key: string): void {
  responseField(value, key, 'bigint');
  if ((value[key] as bigint) < 0n || (value[key] as bigint) > 4294967295n) throw new Error(`Invalid response ordinal ${key}`);
}

function receiptResponse(value: unknown): ActionReceipt {
  const receipt = responseObject(value);
  for (const field of ['commandId', 'eventId', 'committedAt', 'chosenCurrentTime']) responseField(receipt, field, 'string');
  responseField(receipt, 'logSeq', 'bigint');
  for (const rawItem of responseArray(receipt, 'items')) {
    const item = responseObject(rawItem);
    responseOrdinal(item, 'itemOrdinal');
    for (const rawEdit of responseArray(item, 'edits')) {
      const edit = responseObject(rawEdit);
      for (const field of ['ruleOrdinal', 'editOrdinal']) responseOrdinal(edit, field);
      for (const field of ['kind', 'typeName', 'operation']) responseField(edit, field, 'string');
      responseField(edit, 'versionAfter', 'bigint');
      if (!Object.hasOwn(edit, 'pk')) throw new Error('Invalid response: missing pk');
    }
  }
  return value as ActionReceipt;
}

function validationResponse(value: unknown): ValidationResponse {
  if (responseObject(value).validated !== true) throw new Error('Invalid validation response');
  return value as ValidationResponse;
}

function historyResponse(value: unknown): HistoryPage {
  const page = responseObject(value);
  responseField(page, 'truncated', 'boolean');
  if (page.nextBeforeSeq !== undefined && page.nextBeforeSeq !== null) responseField(page, 'nextBeforeSeq', 'bigint');
  for (const rawEntry of responseArray(page, 'entries')) {
    const entry = responseObject(rawEntry);
    for (const field of ['eventId', 'actor', 'action', 'committedAt', 'op']) responseField(entry, field, 'string');
    for (const field of ['seq', 'version']) responseField(entry, field, 'bigint');
    responseObject(entry.values);
    for (const field of ['unset', 'maskedChanges']) {
      if (!responseArray(entry, field).every((item) => typeof item === 'string')) throw new Error(`Invalid response array ${field}`);
    }
  }
  return value as HistoryPage;
}

function asWireJson(value: unknown): WireJson {
  return value as WireJson;
}

export interface OntoforgeClient {
  metadata(options?: CallOptions): Promise<Metadata>;
  get(objectType: string, pk: string, options?: CallOptions): Promise<ObjectRow>;
  query(body: QueryRequest, options?: CallOptions): Promise<ListPage>;
  aggregate(body: AggregateRequest, options?: CallOptions): Promise<AggregateResult>;
  health(options?: CallOptions): Promise<HealthResponse>;
  action(action: string, body: ActionRequest, options?: CallOptions): Promise<ActionReceipt | ValidationResponse>;
  validate(action: string, body: ActionRequest, options?: CallOptions): Promise<ValidationResponse>;
  commandResult(commandId: string, options?: CallOptions): Promise<ActionReceipt>;
  history(objectType: string, pk: string, params?: { limit?: bigint; beforeSeq?: bigint }, options?: CallOptions): Promise<HistoryPage>;
}

export function createOntoforgeClient(options: OntoforgeClientOptions): OntoforgeClient {
  assertLosslessRuntime();
  const fetcher = options.fetch ?? globalThis.fetch;
  const raw = createClient<paths>({
    baseUrl: options.baseUrl,
    headers: options.bearer ? { Authorization: `Bearer ${options.bearer}` } : undefined,
    bodySerializer: (body) => encodeWire(asWireJson(body)),
    fetch: async (request) => {
      assertLosslessRuntime();
      return fetcher(request);
    },
  });

  async function call<T>(response: Promise<{ data?: string; error?: unknown; response: Response }>): Promise<T> {
    const result = await response;
    if (!result.response.ok) {
      let body: unknown = result.error;
      if (typeof body === 'string') {
        try {
          body = decodeWire(body);
        } catch {
          // Keep the original response text in the typed error.
        }
      }
      throw new OntoforgeHttpError(result.response, body);
    }
    return (result.data === undefined ? undefined : decodeWire(result.data)) as T;
  }

  return {
    metadata: (callOptions = {}) => call<Metadata>(raw.GET('/api/ontology', { ...callOptions, parseAs: 'text' })),
    get: (objectType, pk, callOptions = {}) => call<ObjectRow>(raw.GET('/api/objects/{object_type}/{pk}', {
      ...callOptions,
      params: { path: { object_type: objectType, pk } },
      parseAs: 'text',
    })),
    query: (body, callOptions = {}) => call<ListPage>(raw.POST('/api/query', {
      ...callOptions,
      body,
      parseAs: 'text',
    })),
    aggregate: (body, callOptions = {}) => call<AggregateResult>(raw.POST('/api/aggregate', {
      ...callOptions,
      body,
      parseAs: 'text',
    })),
    health: (callOptions = {}) => call<HealthResponse>(raw.GET('/health', { ...callOptions, parseAs: 'text' })),
    action: (action, body, callOptions = {}) => call<unknown>(raw.POST('/api/actions/{action}', {
      ...callOptions,
      params: { path: { action } },
      body,
      parseAs: 'text',
    })).then((value) => body.validateOnly ? validationResponse(value) : receiptResponse(value)),
    validate: (action, body, callOptions = {}) => call<unknown>(raw.POST('/api/actions/{action}', {
      ...callOptions,
      params: { path: { action } },
      body: { ...body, validateOnly: true },
      parseAs: 'text',
    })).then(validationResponse),
    commandResult: (commandId, callOptions = {}) => call<unknown>(raw.GET('/api/commands/{command_id}', {
      ...callOptions,
      params: { path: { command_id: commandId } },
      parseAs: 'text',
    })).then(receiptResponse),
    history: (objectType, pk, params = {}, callOptions = {}) => call<unknown>(raw.GET('/api/objects/{object_type}/{pk}/history', {
      ...callOptions,
      params: { path: { object_type: objectType, pk }, query: params },
      parseAs: 'text',
    })).then(historyResponse),
  };
}
