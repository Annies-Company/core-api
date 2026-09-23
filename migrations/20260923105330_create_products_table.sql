CREATE TABLE IF NOT EXISTS products (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT          NOT NULL,
    description TEXT,
    price       NUMERIC(12,2) NOT NULL CHECK (price >= 0),
    category    TEXT          NOT NULL,
    image_url   TEXT,
    stock       INTEGER       NOT NULL DEFAULT 0 CHECK (stock >= 0),
    created_at  TIMESTAMPTZ   NOT NULL DEFAULT now()
);
