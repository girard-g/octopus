create table link (
    id          uuid primary key default gen_random_uuid(),
    url         text not null,
    title       text not null,
    description text,
    category    text,
    tags        text[] not null default '{}',
    created_at  timestamptz not null default now()
);
create index link_category_idx on link(category);
create index link_tags_idx on link using gin(tags);
