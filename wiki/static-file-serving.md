# Static File Serving

## What It Does

LibreChat now serves its own web interface directly from the backend server.
When you open the app in a browser, the same server that handles API requests
also delivers the HTML, JavaScript, and WASM files that make up the user
interface. This means you only need to run a single program — no separate
frontend server required.

## How to Use It

1. **Build the frontend** (you only need to do this once, or after making UI changes)
   ```bash
   cd frontend && trunk build
   ```
   This creates the compiled files in `frontend/dist/`.

2. **Start the server**
   ```bash
   cargo run -p server
   ```
   The server starts and logs: `Listening on 0.0.0.0:3000`

3. **Open the app**
   - Visit `http://localhost:3000/` in your browser
   - You'll see the Leptos WASM interface load
   - The health check is still at `http://localhost:3000/health`

4. **Use a custom static directory**
   ```bash
   LIBRECHAT_STATIC_DIR=/path/to/my/dist cargo run -p server
   ```
   The server will serve frontend files from that directory instead. Supply an absolute path to avoid surprises if the working directory changes.

## How It Works

The server uses two rules for handling requests:

| Request path          | What happens                                        |
| --------------------- | --------------------------------------------------- |
| `/health`             | Returns `{"status":"ok"}` (API handler)             |
| `/`                   | Serves `index.html` (the app entry point)           |
| `/some-file.js`       | Serves the file from the static directory           |
| `/any/other/path`     | Serves `index.html` (so the app can handle the URL) |

The last rule is called **SPA fallback** — it lets the Leptos app handle
URLs like `/chat/history` in the browser, even though those pages don't
exist as real files on the server.

## Glossary

| Term           | Meaning                                                              |
| -------------- | -------------------------------------------------------------------- |
| **WASM**       | WebAssembly — a fast binary format that runs in the browser          |
| **SPA**        | Single-Page Application — the app loads once, then handles URLs in the browser |
| **ServeDir**   | A tower-http service that serves files from a directory             |
| **Static files** | Files like HTML, JS, CSS, and WASM that don't change per request  |
| **Fallback**   | What the server does when a URL doesn't match any file              |

## FAQ / Troubleshooting

**Q: I see "404 Not Found" when I visit the app.**
A: Make sure you've built the frontend first: `cd frontend && trunk build`.
The server looks for files in `frontend/dist/` by default.

**Q: Can I change where the server looks for frontend files?**
A: Yes — set the `LIBRECHAT_STATIC_DIR` environment variable to the directory
containing your built frontend.

**Q: Why does visiting a random URL like `/chat/history` return a page instead of 404?**
A: That's the SPA fallback. The server returns `index.html` for any unknown path
so the Leptos router in the browser can handle the URL. If the Leptos router
doesn't recognise it either, it will show its own 404 page.

**Q: The app works but `/health` returns HTML instead of JSON — what's wrong?**
A: This shouldn't happen. API routes are registered before the static file
catch-all, so `/health` always returns JSON. If you see HTML, check that the
health route is correctly registered in `lib.rs`.

**Q: Will the static files be embedded in the binary eventually?**
A: Yes — a future issue will embed the assets into the binary using
`include_dir!` or `rust-embed`, eliminating the need for a separate `dist/`
directory at runtime.

## Related Resources

- [Technical docs](../docs/static-file-serving.md)
- [tower-http ServeDir documentation](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html)
- [Leptos framework](https://leptos.dev/)
