# Test-only signing keys

The two PEM files in this directory are freshly generated key pairs that exist only so the
OIDC tests in `tests/oidc.rs` can sign JWTs and publish the matching public keys from a
local mock JWKS server.

| File | Algorithm | Generated with |
|------|-----------|----------------|
| `oidc_rs256.pem` | RS256, RSA 2048, PKCS#8 | `ssh-keygen -t rsa -b 2048 -m PKCS8 -N "" -C ontoforge-test-only` |
| `oidc_es256.pem` | ES256, P-256, PKCS#8 | `ssh-keygen -t ecdsa -b 256 -m PKCS8 -N "" -C ontoforge-test-only` |

They are for tests only and have no real use: no identity provider, service or deployment
trusts them. Never copy them into a configuration, and never reuse them for anything else.
Secret scanners may flag them; that is expected and the findings can be dismissed.
