-- Tables of the Iceberg JDBC catalog (schema version V1), created in the database
-- `iceberg_catalog` of the nightly CI PostgreSQL. Trino's JDBC catalog expects them to exist.
--
-- Usage: psql -v ON_ERROR_STOP=1 -d iceberg_catalog -f ci/pg/iceberg_catalog.sql

CREATE TABLE iceberg_namespace_properties (
    catalog_name varchar(255) NOT NULL,
    namespace varchar(255) NOT NULL,
    property_key varchar(255),
    property_value varchar(1000),
    PRIMARY KEY (catalog_name, namespace, property_key)
);

CREATE TABLE iceberg_tables (
    catalog_name varchar(255) NOT NULL,
    table_namespace varchar(255) NOT NULL,
    table_name varchar(255) NOT NULL,
    metadata_location varchar(1000),
    previous_metadata_location varchar(1000),
    iceberg_type varchar(5),
    PRIMARY KEY (catalog_name, table_namespace, table_name)
);
