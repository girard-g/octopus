create table link (
    id          uuid primary key default gen_random_uuid(),
    url         text not null,
    title       text not null,
    description text,
    category    text,
    tags        text[] not null default '{}',
    created_at  timestamptz not null default now()
);
