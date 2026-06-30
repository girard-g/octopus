create table event (
    id          uuid primary key default gen_random_uuid(),
    title       text not null,
    starts_at   timestamptz not null,
    ends_at     timestamptz not null,
    all_day     boolean not null default false,
    project_id  uuid references project(id) on delete set null,
    contact_id  uuid references contact(id) on delete set null,
    notes       text,
    created_at  timestamptz not null default now()
);
create index event_starts_idx on event(starts_at);
