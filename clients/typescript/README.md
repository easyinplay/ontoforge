# ontoforge

TypeScript client for the [ontoforge](https://github.com/easyinplay/ontoforge) ontology operations REST API.

## Install

```sh
npm install ontoforge
```

## Usage

```ts
import { createOntoforgeClient } from 'ontoforge';

const client = createOntoforgeClient({
  baseUrl: 'http://localhost:8080',
  bearer: '<token>', // optional
});

const health = await client.health();
const page = await client.query({
  set: { kind: 'type', object_type: 'Employee' },
  page: { first: 10n },
});
```

The client preserves 64-bit integers end to end: object ids, sequence numbers and
limits are `bigint` on the wire and in the API surface. It also exports the lossless
wire codec (`encodeWire`, `decodeWire`, `assertLosslessRuntime`,
`LosslessCodecError`), the `OntoforgeHttpError` type, and the generated OpenAPI
`components`/`paths` types.

## Repository

Source and issue tracker: <https://github.com/easyinplay/ontoforge>
(this package lives in `clients/typescript`).

## License

Apache-2.0
