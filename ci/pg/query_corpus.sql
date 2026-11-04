-- Deterministic Phase 4 source rows. Run after ci/pg/seed.sql in database demo.
-- Usage: psql -v ON_ERROR_STOP=1 -d demo -f ci/pg/query_corpus.sql
\set ON_ERROR_STOP on

BEGIN;

INSERT INTO aviation.flights (
    flight_id, flight_no, departure_airport, arrival_airport, tail_number,
    scheduled_departure, status, delay_minutes, tags
) VALUES
    ('Q-0001', 'QF101', 'FRA', 'JFK', 'D-AAAA', '2026-04-01 09:00:00', 'Boarding', 0, ARRAY['red', 'fast']),
    ('Q-0002', 'QF102', 'FRA', 'LHR', 'D-AAAA', '2026-04-01 09:00:00', 'boarding', 10, ARRAY['red']),
    ('Q-0003', 'QF103', 'JFK', 'FRA', 'D-AAAA', NULL, 'delayed', 20, ARRAY['blue', 'fast']),
    ('Q-0004', 'QF104', 'JFK', 'LHR', 'D-AAAA', NULL, 'delayed', NULL, NULL),
    ('Q-0005', 'QF105', 'LHR', 'FRA', 'D-AAAA', '2026-04-01 10:00:00', NULL, 20, ARRAY[]::text[]),
    ('Q-0006', 'QF106', 'FRA', 'JFK', 'D-AAAA', '2026-04-01 10:00:00', 'scheduled', 30, ARRAY['green'])
ON CONFLICT (flight_id) DO UPDATE SET
    flight_no = EXCLUDED.flight_no,
    departure_airport = EXCLUDED.departure_airport,
    arrival_airport = EXCLUDED.arrival_airport,
    tail_number = EXCLUDED.tail_number,
    scheduled_departure = EXCLUDED.scheduled_departure,
    status = EXCLUDED.status,
    delay_minutes = EXCLUDED.delay_minutes,
    tags = EXCLUDED.tags;

INSERT INTO aviation.flight_crew (flight_id, pilot_id) VALUES
    ('Q-0001', 'E-001'), ('Q-0001', 'E-003'), ('Q-0002', 'E-003'),
    ('Q-0003', 'E-001'), ('Q-0004', 'E-003')
ON CONFLICT (flight_id, pilot_id) DO NOTHING;

COMMIT;
