//! Physical names of edit store tables and columns (ADR-0024, ADR-0002 rule 5).
//!
//! Every name is derived from a validated apiName (ASCII letters and digits) by snake casing,
//! so it can never carry a quote, a space or a separator that changes the meaning of a DDL
//! statement. Names longer than PostgreSQL's identifier limit are cut and carry a hash of the
//! full name (`bounded`); collisions between the resulting names are found by
//! `PhysicalSchema::derive` (ONT-V24).

use heck::ToSnakeCase;
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};

/// Prefix of the table of an object type.
pub const OBJECT_TABLE_PREFIX: &str = "of_obj_";
/// Prefix of the join table of a many-to-many link type.
pub const LINK_TABLE_PREFIX: &str = "of_lnk_";
/// Suffix of the boolean column that records "this property was set by an edit".
pub const SET_SUFFIX: &str = "__set";
/// System columns of an object table (ADR-0024 point 4).
pub const OBJECT_SYSTEM_COLUMNS: [&str; 5] = [
    "of_version",
    "of_deleted",
    "of_released",
    "of_def_version",
    "of_log_seq",
];
/// System columns of a link table, which has no `of_released`.
pub const LINK_SYSTEM_COLUMNS: [&str; 4] =
    ["of_version", "of_deleted", "of_def_version", "of_log_seq"];
/// Prefix of a plain index.
pub const INDEX_PREFIX: &str = "of_ix_";
/// Prefix of the partial unique index behind a `oneToOne` link.
pub const UNIQUE_INDEX_PREFIX: &str = "of_ux_";
/// Longest snake cased part of a table name (ADR-0024 point 5).
pub const TABLE_BASE_MAX: usize = 29;
/// Longest snake cased part of a column name.
pub const COLUMN_BASE_MAX: usize = 30;
/// PostgreSQL's identifier limit in bytes; index names are capped at it.
pub const IDENT_MAX: usize = 63;

/// `name` itself when it fits in `max` bytes, else its first `max - 9` bytes, `_` and the 32 bit
/// FNV-1a hash of the whole name as 8 lower case hex digits (ADR-0024 point 5). The hash covers
/// the untruncated name, so two long names that share a prefix stay apart. Names are ASCII (they
/// are snake cased apiNames and prefixes), so cutting at a byte never splits a character.
pub fn bounded(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_owned();
    }
    let keep = max.saturating_sub(9);
    format!("{}_{:08x}", &name[..keep], fnv1a32(name.as_bytes()))
}

/// The published 32 bit FNV-1a constants, written as in the specification.
#[allow(clippy::unreadable_literal, reason = "the published FNV-1a constants")]
const FNV_OFFSET_BASIS: u32 = 0x811c9dc5;
#[allow(clippy::unreadable_literal, reason = "the published FNV-1a constants")]
const FNV_PRIME: u32 = 0x01000193;

fn fnv1a32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// The published 64 bit FNV-1a constants.
#[allow(clippy::unreadable_literal, reason = "the published FNV-1a constants")]
const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
#[allow(clippy::unreadable_literal, reason = "the published FNV-1a constants")]
const FNV64_PRIME: u64 = 0x100000001b3;

/// The 64 bit FNV-1a hash of `bytes` as 16 lower case hex digits. It tells two texts apart, for
/// example the plan that was reviewed from the plan that is about to run; it is not a defence
/// against an adversary.
pub(crate) fn fnv1a64_hex(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(FNV64_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV64_PRIME)
    });
    format!("{hash:016x}")
}

/// Infix of the name a column or table gets when migration archives it: `<name>__arch_v<N>`.
pub const ARCHIVE_INFIX: &str = "__arch_v";

/// Name of the archived value column: `capacity` archived at definition version 2 becomes
/// `capacity__arch_v2` (ADR-0024 point 3).
pub fn archived_column(column: &str, def_version: i64) -> String {
    bounded(&format!("{column}{ARCHIVE_INFIX}{def_version}"), IDENT_MAX)
}

/// Name of the archived `__set` column: `capacity__set__arch_v2`.
pub fn archived_set_column(column: &str, def_version: i64) -> String {
    archived_column(&set_column(column), def_version)
}

/// Name of an archived table: `of_obj_gate__arch_v2`.
pub fn archived_table(table: &str, def_version: i64) -> String {
    archived_column(table, def_version)
}

/// Name of the plain index on `column` of `table`.
pub fn index_name(table: &str, column: &str) -> String {
    bounded(&format!("{INDEX_PREFIX}{table}_{column}"), IDENT_MAX)
}

/// Name of the partial unique index on `column` of `table`.
pub fn unique_index_name(table: &str, column: &str) -> String {
    bounded(&format!("{UNIQUE_INDEX_PREFIX}{table}_{column}"), IDENT_MAX)
}

/// Table of an object type, for example `of_obj_gate_stand` for `GateStand`.
pub fn object_table(name: &ObjectTypeName) -> String {
    format!(
        "{OBJECT_TABLE_PREFIX}{}",
        bounded(&name.as_str().to_snake_case(), TABLE_BASE_MAX)
    )
}

/// Join table of a many-to-many link type, for example `of_lnk_gate_stands` for `gateStands`.
pub fn link_table(name: &LinkTypeName) -> String {
    format!(
        "{LINK_TABLE_PREFIX}{}",
        bounded(&name.as_str().to_snake_case(), TABLE_BASE_MAX)
    )
}

/// Column of a property, for example `max_load` for `maxLoad`.
pub fn property_column(name: &PropertyName) -> String {
    bounded(&name.as_str().to_snake_case(), COLUMN_BASE_MAX)
}

/// The "was set" flag column of `column`, for example `max_load__set`.
pub fn set_column(column: &str) -> String {
    format!("{column}{SET_SUFFIX}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(s: &str) -> ObjectTypeName {
        ObjectTypeName::try_from(s.to_owned()).expect("object name")
    }

    fn property(s: &str) -> PropertyName {
        PropertyName::try_from(s.to_owned()).expect("property name")
    }

    fn link(s: &str) -> LinkTypeName {
        LinkTypeName::try_from(s.to_owned()).expect("link name")
    }

    #[test]
    fn bounded_names() {
        assert_eq!(bounded("short", TABLE_BASE_MAX), "short");
        let exact = "a".repeat(TABLE_BASE_MAX);
        assert_eq!(bounded(&exact, TABLE_BASE_MAX), exact);

        let long = format!("p{}", "x".repeat(99));
        let table = object_table(&object(&format!("T{}", "x".repeat(99))));
        assert_eq!(
            table.len(),
            OBJECT_TABLE_PREFIX.len() + TABLE_BASE_MAX,
            "{table}"
        );
        let column = property_column(&property(&long));
        assert_eq!(column.len(), COLUMN_BASE_MAX);
        let (head, hash) = column.split_at(COLUMN_BASE_MAX - 8);
        assert!(head.ends_with('_'), "{column}");
        assert!(
            hash.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{column}"
        );

        assert_eq!(bounded(&long, COLUMN_BASE_MAX), column, "deterministic");
        let other = format!("p{}y", "x".repeat(98));
        assert_ne!(
            bounded(&other, COLUMN_BASE_MAX),
            column,
            "only the tail differs"
        );

        // The published FNV-1a test vector for "a".
        assert_eq!(fnv1a32(b"a"), 0xe40c_292c);
    }

    #[test]
    fn archive_names_carry_the_definition_version() {
        assert_eq!(archived_column("capacity", 2), "capacity__arch_v2");
        assert_eq!(archived_set_column("capacity", 2), "capacity__set__arch_v2");
        assert_eq!(archived_table("of_obj_gate", 12), "of_obj_gate__arch_v12");
        let long = archived_column(&"c".repeat(30), 10);
        assert!(long.len() <= IDENT_MAX, "{long}");
        let long_set = archived_set_column(&"c".repeat(60), 10);
        assert!(long_set.len() <= IDENT_MAX, "{long_set}");
        assert_ne!(
            archived_column(&"c".repeat(70), 2),
            archived_column(&"c".repeat(70), 3)
        );
    }

    #[test]
    fn fnv1a64_matches_the_published_vectors() {
        assert_eq!(fnv1a64_hex(b""), "cbf29ce484222325");
        assert_eq!(fnv1a64_hex(b"a"), "af63dc4c8601ec8c");
        assert_eq!(fnv1a64_hex(b"foobar"), "85944171f73967e8");
    }

    #[test]
    fn index_names_stay_within_the_postgres_limit() {
        assert_eq!(
            index_name("of_obj_flight", "status"),
            "of_ix_of_obj_flight_status"
        );
        let table = format!("of_obj_{}", "t".repeat(29));
        let column = "c".repeat(35);
        assert!(index_name(&table, &column).len() <= IDENT_MAX);
        assert!(unique_index_name(&table, &column).len() <= IDENT_MAX);
        assert!(unique_index_name(&table, &column).starts_with("of_ux_"));
    }

    #[test]
    fn names_are_snake_cased_with_prefixes() {
        assert_eq!(object_table(&object("Gate")), "of_obj_gate");
        assert_eq!(object_table(&object("GateStand")), "of_obj_gate_stand");
        assert_eq!(link_table(&link("gateStands")), "of_lnk_gate_stands");
        assert_eq!(property_column(&property("maxLoad")), "max_load");
        assert_eq!(set_column("max_load"), "max_load__set");
    }
}
