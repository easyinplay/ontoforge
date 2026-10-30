import createClient, { type Client } from 'openapi-fetch';
import type { components, paths } from './generated.js';
import { decodeWire, encodeWire, type WireJson } from './codec.js';

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

function asWireJson(value: unknown): WireJson {
  return value as WireJson;
}

export interface OntoforgeClient {
  readonly raw: Client<paths>;
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
  const fetcher = options.fetch ?? globalThis.fetch;
  const raw = createClient<paths>({
    baseUrl: options.baseUrl,
    headers: options.bearer ? { Authorization: `Bearer ${options.bearer}` } : undefined,
    bodySerializer: (body) => encodeWire(asWireJson(body)),
    fetch: (request) => fetcher(request),
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
    raw,
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
    action: (action, body, callOptions = {}) => call<ActionReceipt | ValidationResponse>(raw.POST('/api/actions/{action}', {
      ...callOptions,
      params: { path: { action } },
      body,
      parseAs: 'text',
    })),
    validate: (action, body, callOptions = {}) => call<ValidationResponse>(raw.POST('/api/actions/{action}', {
      ...callOptions,
      params: { path: { action } },
      body: { ...body, validateOnly: true },
      parseAs: 'text',
    })),
    commandResult: (commandId, callOptions = {}) => call<ActionReceipt>(raw.GET('/api/commands/{command_id}', {
      ...callOptions,
      params: { path: { command_id: commandId } },
      parseAs: 'text',
    })),
    history: (objectType, pk, params = {}, callOptions = {}) => call<HistoryPage>(raw.GET('/api/objects/{object_type}/{pk}/history', {
      ...callOptions,
      params: { path: { object_type: objectType, pk }, query: params },
      parseAs: 'text',
    })),
  };
}
