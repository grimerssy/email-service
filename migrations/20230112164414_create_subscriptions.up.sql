create table subscriptions (
    id uuid primary key,
    name text not null,
    email text not null unique,
    subscribed_at timestamptz not null
);
