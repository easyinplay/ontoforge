const assert = require("node:assert/strict");
const raw = '{"n":9007199254740993}';
assert.equal(JSON.parse(raw).n, 9007199254740992);
const value = JSON.parse(raw, (_key, item, context) =>
  typeof item === "number" && Number.isInteger(item) && !Number.isSafeInteger(item)
    ? BigInt(context.source)
    : item,
);
assert.equal(value.n, 9007199254740993n);
const roundTrip = JSON.stringify(value, (_key, item) =>
  typeof item === "bigint" ? JSON.rawJSON(item.toString()) : item,
);
assert.equal(roundTrip, raw);
console.log(JSON.stringify({ ordinary: JSON.parse(raw).n, lossless: String(value.n), roundTrip }));
