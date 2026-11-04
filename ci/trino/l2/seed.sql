CREATE SCHEMA hive.sales;
CREATE TABLE hive.sales.orders AS SELECT 1 id, ARRAY[7] arr, CAST(ROW(ROW(ROW(9))) AS ROW(b ROW(c ROW(d INTEGER)))) a;
CREATE TABLE hive.sales.shipments AS SELECT 1 id;
CREATE TABLE hive.sales.items AS SELECT 1 id;
CREATE TABLE hive.sales."ordеrs" AS SELECT 2 id;
