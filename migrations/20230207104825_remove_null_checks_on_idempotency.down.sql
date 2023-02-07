alter table idempotency alter column response_status_code set not null;
alter table idempotency alter column response_body set not null;
alter table idempotency alter column response_headers set not null;
