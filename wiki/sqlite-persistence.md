# SQLite Persistence Foundation

**What it does:**  
LibreChat now remembers your conversations, messages, and preferences by
storing them in a local SQLite database file instead of keeping everything in
memory.  When you restart the server, your chat threads and settings survive.

---

## Feature Overview

Before this change, LibreChat lost all chat history and settings every time
the server restarted.  The SQLite persistence foundation introduces a small,
local database that lives on disk right next to the application.  It is the
first building block of Phase 2; future updates will add a full chat-history
UI and richer settings screens on top of this foundation.

### Who benefits?
- **End users** who want their chat history to persist across server restarts.
- **Self-hosters** running LibreChat on a Raspberry Pi or low-power device,
  because SQLite is extremely lightweight and requires no separate database
  server.

---

## User Guide

### Starting the server for the first time

No manual setup is required.  Simply run the server as usual:

```bash
cargo run -p server
```

On first startup LibreChat will:
1. Create a file named `librechat.db` in the working directory.
2. Set up the tables needed for conversations, messages, and preferences.

### Using a custom database location

If you want the database file somewhere else (for example, on a larger disk
or in a backup-friendly path), set the environment variable before starting:

```bash
export LIBRECHAT_DATABASE_URL="sqlite:/path/to/my/librechat.db"
cargo run -p server
```

### What is stored?

| Data | Table | Notes |
|------|-------|-------|
| Chat threads | `conversations` | One row per thread, with a title and timestamps. |
| Individual messages | `messages` | Linked to a conversation; stores who sent it (`user` or `assistant`) and the text. |
| App theme / language | `user_settings` | Global preferences (single row). |
| Preferred model | `model_preferences` | Which provider and model to use by default (single row). |

---

## Glossary

| Term | Meaning |
|------|---------|
| **SQLite** | A small, file-based database engine built into the application.  No separate installation needed. |
| **SQLx** | The Rust library used to talk to SQLite.  It checks SQL queries for correctness while the code is being built. |
| **Migration** | A script that creates or updates database tables.  Migrations run automatically when the server starts. |
| **Pool** | A group of reusable database connections.  It makes the server faster and more reliable under load. |

---

## FAQ / Troubleshooting

### Q: The server fails to start with a "database initialisation" error.  
**A:** Make sure the directory where the database file lives is writable.  If
you set a custom `LIBRECHAT_DATABASE_URL`, check that the parent directory
exists.  The default path `librechat.db` requires write permission in the
folder where you run the server.

### Q: Can I use PostgreSQL or MySQL instead?  
**A:** Not with this issue.  The foundation is intentionally SQLite-only to
keep the project small and portable.  Remote database support may be explored
in a later Phase.

### Q: Where is my data?  
**A:** By default, in a file called `librechat.db` in the same folder you
start the server from.  You can open it with any SQLite viewer or the
`sqlite3` command-line tool.

### Q: Do I need to run migration scripts manually?  
**A:** No.  Migrations run automatically every time the server starts.

### Q: Will the database work on a Raspberry Pi?  
**A:** Yes.  SQLite is one of the most resource-efficient databases
available and is widely used on embedded devices.

---

## Related Resources

- **Technical docs:** [`docs/sqlite-persistence.md`](../docs/sqlite-persistence.md)
- **SQLx compile-time query docs:** https://docs.rs/sqlx/latest/sqlx/macro.query.html
- **SQLite syntax:** https://www.sqlite.org/lang.html
- **Next planned feature:** Chat history UI (issue coming in Phase 2)
