alter table event add column series_id uuid;
create index event_series_idx on event(series_id);
