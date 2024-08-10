-- Add migration script here
ALTER TABLE members
ADD personal_info jsonb;
