alter table task add column priority    text check (priority in ('low','medium','high'));
alter table task add column size        text check (size in ('xs','s','m','l','xl'));
alter table task add column description text;
alter table task add column checklist   jsonb not null default '[]';
alter table task add column position    integer not null default 0;
