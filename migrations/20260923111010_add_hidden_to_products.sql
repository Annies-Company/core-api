-- Add migration script here
ALTER TABLE products
    ADD COLUMN hidden BOOLEAN NOT NULL DEFAULT false;