# Health Check Server

## What It Does

LibreChat now runs a small backend server that answers a simple question: *"Is the
server alive?"* When you start it, the server listens on a port and responds to a
health check request. This is the foundation that future features (like the
chat API and static file serving) will build on.

## How to Use It

1. **Start the server**
   ```bash
   cargo run -p server
   ```
   The server starts and logs: `Listening on 127.0.0.1:3000`

2. **Check health**
   ```bash
   curl http://localhost:3000/health
   ```
   You'll get:
   ```json
   {"status":"ok"}
   ```

3. **Use a custom port**
   ```bash
   LIBRECHAT_PORT=8080 cargo run -p server
   ```
   The server now listens on port 8080.

## Glossary

| Term                | Meaning                                                        |
| ------------------- | -------------------------------------------------------------- |
| **Health check**    | A simple endpoint that confirms the server is running            |
| **CORS**            | Cross-Origin Resource Sharing — lets browsers call the API     |
| **Tracing**         | Structured logging that shows request details in the console   |
| **AppState**        | Shared data available to all request handlers, including config like provider/static paths |

## FAQ / Troubleshooting

**Q: Why does the server log look different now?**
A: We added structured request logging (`tracing`). Every HTTP request produces
a log line with method, path, status, and latency.

**Q: Can I change what port the server uses?**
A: Yes — set the `LIBRECHAT_PORT` environment variable before starting.

**Q: The health check returns something other than `{"status":"ok"}` — is that a bug?**
A: That would be a bug. The only valid response is `{"status":"ok"}` with HTTP 200
and `Content-Type: application/json`.

**Q: Why isn’t my browser origin allowed?**
A: CORS now uses an allowlist. By default it allows common localhost
development origins. For other origins, set `LIBRECHAT_ALLOWED_ORIGINS` to a
comma-separated list such as `https://app.example.com,https://admin.example.com`.

## Related Resources

- [Technical docs](../docs/server-health-check.md)
- [Axum documentation](https://docs.rs/axum)
- [tower-http CORS middleware](https://docs.rs/tower-http/latest/tower_http/cors/)
