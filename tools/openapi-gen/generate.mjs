import { readFile, writeFile } from 'node:fs/promises';
import openapiTS, { astToString } from 'openapi-typescript';
import ts from 'typescript';

const [, , input = '../../docs/openapi.json', output = '../../clients/typescript/src/generated.ts'] = process.argv;
const document = JSON.parse(await readFile(new URL(input, import.meta.url), 'utf8'));
const nodes = await openapiTS(document, {
  transform(schema) {
    if (schema.type === 'integer' || (Array.isArray(schema.type) && schema.type.includes('integer'))) {
      const bigint = ts.factory.createKeywordTypeNode(ts.SyntaxKind.BigIntKeyword);
      return Array.isArray(schema.type) ? ts.factory.createUnionTypeNode([bigint, ts.factory.createLiteralTypeNode(ts.factory.createNull())]) : bigint;
    }
    return undefined;
  },
});
const source = nodes.map((node) => astToString(node)).join('\n');
await writeFile(new URL(output, import.meta.url), source, 'utf8');
