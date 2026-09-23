-- Add migration script here
CREATE TABLE IF NOT EXISTS orders (
    id          TEXT          PRIMARY KEY,
    total       NUMERIC(12,2) NOT NULL CHECK (total >= 0),
    status      TEXT          NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS order_items (
    id          BIGSERIAL     PRIMARY KEY,
    order_id    TEXT          NOT NULL REFERENCES orders(id),
    product_id  BIGINT        NOT NULL REFERENCES products(id),
    size        TEXT          NOT NULL CHECK (size IN ('small', 'medium', 'large')),
    quantity    INTEGER       NOT NULL CHECK (quantity > 0),
    unit_price  NUMERIC(12,2) NOT NULL CHECK (unit_price >= 0),
    line_total  NUMERIC(12,2) NOT NULL CHECK (line_total >= 0)
);

ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_items ENABLE ROW LEVEL SECURITY;