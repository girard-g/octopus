create table note (
    id          uuid primary key default gen_random_uuid(),
    body        text not null,
    contact_id  uuid references contact(id) on delete cascade,
    project_id  uuid references project(id) on delete cascade,
    created_at  timestamptz not null default now(),
    constraint note_one_parent check (
        (contact_id is not null)::int + (project_id is not null)::int = 1
    )
);
create index note_contact_idx on note(contact_id);
create index note_project_idx on note(project_id);
