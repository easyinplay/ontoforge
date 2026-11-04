-- Seeds the fixed row predicate corpus for the Trino differential test (CI only).
--
-- Usage (from the repository root):
--   psql -v ON_ERROR_STOP=1 -d demo -f ci/pg/rowpred_corpus.sql
--
-- The rows below are the `rows` array of
-- crates/ontoforge-auth/tests/data/rowpred_corpus.json, value for value. The two files must
-- stay in sync: the first assertion of crates/ontoforge-auth/tests/rowpred_trino_it.rs reads
-- `demo.rowpred.rows` through Trino and compares it with the JSON, so a drift fails the test.
-- Text columns use COLLATE "C" so that `<` and `>` on strings compare UTF-8 bytes, as the
-- Rust evaluator and the SQLite stand-in do (ADR-0025).
--
-- The script is not idempotent: it creates the schema, so run it once per database.

\set ON_ERROR_STOP on

BEGIN;

CREATE SCHEMA rowpred;

CREATE TABLE rowpred.rows (
    id integer PRIMARY KEY,
    b boolean,
    i bigint,
    d double precision,
    s text COLLATE "C",
    t text COLLATE "C"
);

INSERT INTO rowpred.rows (id, b, i, d, s, t) VALUES
    (1, true, 0, 0.0, 'acme', 'acme'),
    (2, false, 1, 1.5, 'ACME', 'Acme'),
    (3, NULL, NULL, NULL, NULL, NULL),
    (4, true, -1, -1.5, 'acme corp', 'corp'),
    (5, false, 9223372036854775807, 1E+300, 'abc', 'ABC'),
    (6, true, -9223372036854775808, -1E+300, 'ab', NULL),
    (7, false, 2, 0.5, 'a%b', 'a_b'),
    (8, true, 3, 2.25, 'a\b', 'a\'),
    (9, false, 4, -0.5, 'a[b]c', '[*?]'),
    (10, true, 5, 3.5, 'a*c', 'a?c'),
    (11, false, 6, 100.0, 'it''s', ''''),
    (12, true, 7, 99.5, '', ''),
    (13, false, 8, 0.1, 'É', 'é'),
    (14, true, 9, 1E-7, '😀 emoji', '😀'),
    (15, NULL, 100, 100.0, 'eu', 'us'),
    (16, false, 101, 99.5, 'us', 'eu'),
    (17, true, NULL, 5.0, '%', '_'),
    (18, NULL, 10, NULL, '\%', '\_'),
    (19, false, 11, 7.75, 'eu-west', 'EU-WEST'),
    (20, true, 12, -2.0, NULL, 'acme'),
    (21, NULL, 13, 8.0, 'acme', NULL),
    (22, false, -100, NULL, 'Zed', 'zed'),
    (23, true, 14, 6.0, 'b', 'a'),
    (24, false, 15, 4.0, 'a b', 'ab ');

COMMIT;
