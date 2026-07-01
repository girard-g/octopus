-- Projects can be created without a contact; assign one later.
alter table project alter column contact_id drop not null;
