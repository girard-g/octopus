## Deploy (Coolify)

1. Create a **Postgres** resource in Coolify. Note its internal connection URL.
2. Create an **Application** from this Git repo, build pack = Dockerfile.
3. Set environment variables:
   - `DATABASE_URL` = the Coolify Postgres internal URL
   - `APP_PASSWORD` = your login password
   - `SESSION_SECRET` = 64+ random bytes (`openssl rand -hex 48`)
   - `PORT` = `8080`
4. Set the app port to `8080`. Deploy. Migrations run automatically on boot.

## Local development

1. Start a local Postgres (e.g. Docker):

   ```bash
   docker run -d --name octopus-pg -p 5432:5432 \
     -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=postgres \
     postgres:16
   ```

2. Create the `octopus` role + database it owns (matches `.env.example`). The role
   must OWN the database, or `sqlx` migrations fail with `permission denied for
   table _sqlx_migrations`:

   ```bash
   PGPASSWORD=postgres psql -h localhost -U postgres -c \
     "CREATE ROLE octopus WITH LOGIN PASSWORD 'octopus';"
   PGPASSWORD=postgres psql -h localhost -U postgres -c \
     "CREATE DATABASE octopus OWNER octopus;"
   ```

3. Copy `.env.example` to `.env` and set **real** values:
   - `DATABASE_URL=postgres://octopus:octopus@localhost:5432/octopus`
   - `SESSION_SECRET` — must NOT be left as the example placeholder (the server
     rejects it at boot). Generate one: `openssl rand -hex 48`.
   - `APP_PASSWORD` — your login password (not empty).

4. `cargo run`. Migrations run automatically on boot.

Tests: `cargo test` needs `DATABASE_URL` pointing at a Postgres login role that can
create databases (e.g. `postgres://postgres:postgres@localhost:5432/postgres`).
`#[sqlx::test]` creates isolated databases per test.

## API notes

- All `PUT /api/{resource}/{id}` endpoints are **full-replace**: send the
  complete object. Any field omitted from the body is reset to its
  default/null (e.g. omitting `email` on a contact PUT clears it). The intended
  client flow is fetch → modify → send the whole object back.
- **Exception:** a project's `status` and `board_order` are NOT changed by
  `PUT /api/projects/{id}`; they are owned by `PATCH /api/projects/{id}/move`.
  A project PUT preserves the existing pipeline status.
