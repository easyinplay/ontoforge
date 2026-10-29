# HTTP API

`ontoforge serve --server-config FILE --bind 127.0.0.1:8080` starts the authenticated
HTTP host. The current endpoint is `GET /api/objects/{object_type}/{pk}`. Send the
access token in `Authorization: Bearer TOKEN`. Development tokens are accepted only
when the host is explicitly started with `--dev` and `[auth.dev] key_file` is configured.

## Persistent Query Signing Key

The host requires a trusted file reference in the server configuration:

```toml
[query]
signing_key_file = "keys/query-signing.key"
```

The path is relative to the configuration file. The file contains raw bytes, from
32 bytes through 64 KiB. Bytes are not decoded as text or trimmed. Keep this key
independent of OIDC credentials and the development authentication key. A missing,
unreadable, short or oversized file prevents the host from listening. Other CLI
commands continue to accept configurations without `[query]`.

Generate a key once in a private directory, outside version control. For the checked-in
development configuration, run this PowerShell snippet from the repository root:

```powershell
$directory = [System.IO.Path]::GetFullPath('target/ontoforge-dev')
[System.IO.Directory]::CreateDirectory($directory) | Out-Null
$path = [System.IO.Path]::Combine($directory, 'query-signing.key')
$file = [System.IO.File]::Open($path, [System.IO.FileMode]::CreateNew)
try {
    $key = [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)
    $file.Write($key, 0, $key.Length)
} finally {
    $file.Dispose()
}
```

`CreateNew` refuses to overwrite an existing key. Restrict the directory ACL to the
service account; on Unix, use a private directory and mode `0600` for the file.
The command prints no key bytes. Do not place the key in TOML, command-line arguments,
HTTP requests or logs.

Reuse the same file on normal restarts so existing `_version` tokens remain verifiable.
Changing the file invalidates tokens signed by the previous key. The host does not
create or rotate this key. Automatic development initialization is owned by plan `07-12`.

## Reads And Shutdown

Responses contain the logical object row, including its `_version`. `x-request-id`
matches the durable read audit ID after the query has reserved an audit entry.
Authentication and early validation errors use a server-generated transport ID.
Errors contain `code`, `apiName` and `request_id`. Client-supplied principal, roles,
scope and limits are rejected.

Ctrl+C stops accepting connections, allows active requests to finish and explicitly
drains the read audit. Listener and audit drain failures return a nonzero exit code.
Prepare the definition, scope and persistent edit store with the existing CLI commands
before starting this host. Full development seeding and reset are part of plan `07-12`.
