alter table link add column favorite boolean not null default false;
create index link_favorite_idx on link(favorite) where favorite;
