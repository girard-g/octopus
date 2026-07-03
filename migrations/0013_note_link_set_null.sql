alter table note drop constraint note_contact_id_fkey;
alter table note add constraint note_contact_id_fkey foreign key (contact_id) references contact(id) on delete set null;
alter table note drop constraint note_project_id_fkey;
alter table note add constraint note_project_id_fkey foreign key (project_id) references project(id) on delete set null;
