-- nested folders
create table folder (
    id         uuid primary key default gen_random_uuid(),
    name       text not null,
    parent_id  uuid references folder(id) on delete cascade,
    position   int  not null default 0,
    created_at timestamptz not null default now()
);
create index folder_parent_idx on folder(parent_id);

-- extend note: optional title, folder home, pin, updated_at
alter table note add column title      text;
alter table note add column folder_id  uuid references folder(id) on delete set null;
alter table note add column pinned      boolean not null default false;
alter table note add column updated_at  timestamptz not null default now();
create index note_folder_idx on note(folder_id);

-- notes are no longer forced to hang off exactly one parent
alter table note drop constraint note_one_parent;

-- backfill updated_at for existing rows; contact_id/project_id left untouched
update note set updated_at = created_at;
