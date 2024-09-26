-- Add up migration script here
create extension if not exists "uuid-ossp";

create or replace function set_updated_at()
  returns trigger
  as $$
begin
  new.updated_at = now();
  return NEW;
end;
$$
language plpgsql;

-- automate trigger creation
create or replace function trigger_updated_at(tablename regclass)
  returns void
  as $$
begin
  execute format('create trigger set_updated_at
        before update
        on %s
        for each row
        when (OLD is distinct from NEW)
    execute function set_updated_at();', tablename);
end;
$$
language plpgsql;

create table if not exists users(
  id uuid not null default uuid_generate_v4() primary key,
  created_at timestamptz not null default now(),
  updated_at timestamptz,
  name text not null
);

select
  trigger_updated_at('users');

