## Deploy (Coolify)

1. Create a **Postgres** resource in Coolify. Note its internal connection URL.
2. Create an **Application** from this Git repo, build pack = Dockerfile.
3. Set environment variables:
   - `DATABASE_URL` = the Coolify Postgres internal URL
   - `APP_PASSWORD` = your login password
   - `SESSION_SECRET` = 64+ random bytes (`openssl rand -hex 48`)
   - `PORT` = `8080`
4. Set the app port to `8080`. Deploy. Migrations run automatically on boot.

Local dev: copy `.env.example` to `.env`, run a local Postgres, then `cargo run`.
Run `cargo test` against a local Postgres (it creates isolated test databases).
