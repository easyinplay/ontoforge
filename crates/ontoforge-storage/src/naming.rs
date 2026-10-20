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
        assert_eq!(fnv1a32(b"a"), 0xe40c292c);
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
