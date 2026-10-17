-- Seeds the aviation example into the current PostgreSQL database (CI only).
--
-- Usage (from the repository root):
--   psql -v ON_ERROR_STOP=1 -v fixtures="$PWD/examples/ontologies/aviation/bindings/dev/fixtures" \
--        -d demo -f ci/pg/seed.sql
--
-- The fixture CSV files are the only data source: each file is copied into a text staging
-- table and converted into the typed table, so the stand-in and the CI table hold the same
-- rows. Column types follow the type table in docs/source-binding.md (D-13) so that
-- `ontoforge validate --online` against this database is clean. An empty CSV cell is NULL.
--
-- `\copy` does not expand psql variables, so the script changes into the fixture directory
-- and names the files relative to it.

\set ON_ERROR_STOP on
\cd :'fixtures'

BEGIN;

CREATE SCHEMA aviation;

-- Aircraft
CREATE TEMP TABLE stage_aircraft (
    tail_number text,
    model text,
    seat_capacity text,
    in_service_since text
) ON COMMIT DROP;
\copy stage_aircraft FROM 'aircraft.csv' WITH (FORMAT csv, HEADER true)

CREATE TABLE aviation.aircraft (
    tail_number varchar(16) PRIMARY KEY,
    model text NOT NULL,
    seat_capacity integer,
    in_service_since date
);
INSERT INTO aviation.aircraft
SELECT tail_number, model, seat_capacity::integer, in_service_since::date
FROM stage_aircraft;

-- Flight
CREATE TEMP TABLE stage_flights (
    flight_id text,
    flight_no text,
    departure_airport text,
    arrival_airport text,
    tail_number text,
    scheduled_departure text,
    status text,
    delay_minutes text,
    tags text
) ON COMMIT DROP;
\copy stage_flights FROM 'flights.csv' WITH (FORMAT csv, HEADER true)

CREATE TABLE aviation.flights (
    flight_id varchar(16) PRIMARY KEY,
    flight_no varchar(16) NOT NULL,
    departure_airport varchar(3) NOT NULL,
    arrival_airport varchar(3) NOT NULL,
    tail_number varchar(16),
    -- Wall-clock time without a zone; the binding reads it as Europe/Berlin.
    scheduled_departure timestamp(3),
    status text,
    delay_minutes integer,
    tags text[]
);
INSERT INTO aviation.flights
SELECT
    flight_id,
    flight_no,
    departure_airport,
    arrival_airport,
    tail_number,
    scheduled_departure::timestamp(3),
    status,
    delay_minutes::integer,
    -- The fixture writes an array as JSON array text.
    CASE
        WHEN tags IS NULL THEN NULL
        ELSE ARRAY(
            SELECT element
            FROM jsonb_array_elements_text(tags::jsonb) WITH ORDINALITY AS t(element, position)
            ORDER BY position
        )
    END
FROM stage_flights;

-- Pilot
CREATE TEMP TABLE stage_pilots (
    employee_id text,
    full_name text,
    license_number text,
    flight_hours text
) ON COMMIT DROP;
\copy stage_pilots FROM 'pilots.csv' WITH (FORMAT csv, HEADER true)

CREATE TABLE aviation.pilots (
    employee_id varchar(16) PRIMARY KEY,
    full_name text NOT NULL,
    -- Kept in camelCase like the fixture header; Trino exposes it in lower case.
    "licenseNumber" varchar(32),
    flight_hours double precision
);
INSERT INTO aviation.pilots
SELECT employee_id, full_name, license_number, flight_hours::double precision
FROM stage_pilots;

-- MaintenanceRecord
CREATE TEMP TABLE stage_maintenance_records (
    record_id text,
    tail_number text,
    performed_at text,
    performed_by text,
    summary text,
    cost text
) ON COMMIT DROP;
\copy stage_maintenance_records FROM 'maintenance_records.csv' WITH (FORMAT csv, HEADER true)

CREATE TABLE aviation.maintenance_records (
    record_id varchar(16) PRIMARY KEY,
    tail_number varchar(16) NOT NULL,
    -- Carries its own offset; the binding declares `timezone: aware`.
    performed_at timestamptz(3),
    performed_by varchar(16),
    summary text,
    cost numeric(12, 2)
);
INSERT INTO aviation.maintenance_records
SELECT
    record_id,
    tail_number,
    performed_at::timestamptz(3),
    performed_by,
    summary,
    cost::numeric(12, 2)
FROM stage_maintenance_records;

-- flightCrew (many-to-many join table)
CREATE TEMP TABLE stage_flight_crew (
    flight_id text,
    pilot_id text
) ON COMMIT DROP;
\copy stage_flight_crew FROM 'flight_crew.csv' WITH (FORMAT csv, HEADER true)

CREATE TABLE aviation.flight_crew (
    flight_id varchar(16) NOT NULL,
    pilot_id varchar(16) NOT NULL,
    PRIMARY KEY (flight_id, pilot_id)
);
INSERT INTO aviation.flight_crew
SELECT flight_id, pilot_id
FROM stage_flight_crew;

COMMIT;
