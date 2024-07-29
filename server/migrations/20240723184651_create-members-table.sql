DO $$ BEGIN
    CREATE TYPE gender AS ENUM ('male', 'female');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS members
(
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    gender gender NOT NULL,
    birthday TIMESTAMPTZ,
    last_name TEXT NOT NULL,
    image BYTEA,
    mother_id INTEGER,
    father_id INTEGER,

   CONSTRAINT fk_mother
      FOREIGN KEY(mother_id)
        REFERENCES members(id),
   CONSTRAINT fk_father
      FOREIGN KEY(father_id)
        REFERENCES members(id)
);
