-- Add up migration script here
CREATE TABLE IF NOT EXISTS products (
	id SERIAL PRIMARY KEY,
	name VARCHAR NOT NULL,
	price VARCHAR NOT NULL,
	old_price VARCHAR,
	link VARCHAR,
	scraped_at DATE NOT NULL DEFAULT CURRENT_DATE	
);
