import type { OntoforgeHttpError } from '../src/index.js';

export function actionDiagnostic(error: OntoforgeHttpError): [string | null | undefined, bigint | undefined] {
  return [error.envelope?.message, error.envelope?.parameterErrors?.[0]?.itemOrdinal];
}
