create table subscription_tokens(
    subscription_token text primary key,
    subscriber_id uuid not null references subscriptions(id)
);
