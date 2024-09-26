-- Add down migration script here
drop trigger if exists set_updated_at on users;

drop table if exists users;

drop function if exists trigger_updated_at(regclass);

drop function if exists set_updated_at();

