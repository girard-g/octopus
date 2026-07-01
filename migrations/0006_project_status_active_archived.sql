-- Collapse the sales-funnel statuses onto active/archived and drop pipeline ordering.
alter table project drop constraint if exists project_status_check;

update project set status = case
    when status in ('done', 'lost') then 'archived'
    else 'active'
end;

alter table project alter column status set default 'active';
alter table project add constraint project_status_check
    check (status in ('active', 'archived'));

alter table project drop column board_order;
