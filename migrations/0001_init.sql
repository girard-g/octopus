create table contact (
    id          uuid primary key default gen_random_uuid(),
    kind        text not null check (kind in ('person', 'company')),
    name        text not null,
    email       text,
    phone       text,
    company_id  uuid references contact(id) on delete set null,
    created_at  timestamptz not null default now()
);

create table project (
    id          uuid primary key default gen_random_uuid(),
    contact_id  uuid not null references contact(id) on delete cascade,
    title       text not null,
    status      text not null default 'lead'
                check (status in ('lead', 'proposal', 'active', 'done', 'lost')),
    description text,
    invoice_url text,
    board_order integer not null default 0,
    created_at  timestamptz not null default now()
);

create table task (
    id          uuid primary key default gen_random_uuid(),
    project_id  uuid references project(id) on delete cascade,
    title       text not null,
    status      text not null default 'todo'
                check (status in ('todo', 'doing', 'done')),
    due_on      date,
    created_at  timestamptz not null default now()
);

create index project_contact_idx on project(contact_id);
create index project_status_idx  on project(status);
create index task_project_idx    on task(project_id);
