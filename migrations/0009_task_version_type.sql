alter table task add column version text;
alter table task add column type    text check (type in ('feature','bug','enhancement','chore','docs'));
