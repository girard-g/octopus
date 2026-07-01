create table event_contact (
    event_id   uuid not null references event(id)   on delete cascade,
    contact_id uuid not null references contact(id) on delete cascade,
    primary key (event_id, contact_id)
);
create index event_contact_contact_idx on event_contact(contact_id);

-- carry the existing single link forward, then retire the column
insert into event_contact (event_id, contact_id)
    select id, contact_id from event where contact_id is not null;
alter table event drop column contact_id;
