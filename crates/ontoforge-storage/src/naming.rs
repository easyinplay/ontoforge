//! Physical names of edit store tables and columns (ADR-0024, ADR-0002 rule 5).
//!
//! Every name is derived from a validated apiName (ASCII letters and digits) by snake casing,
//! so it can never carry a quote, a space or a separator that changes the meaning of a DDL
//! statement. Length caps and collision checks follow in plan 03-10.

use heck::ToSnakeCase;
use ontoforge_schema::ir::{LinkTypeName, ObjectTypeName, PropertyName};

/// Prefix of the table of an object type.
pub const OBJECT_TABLE_PREFIX: &str = "of_obj_";
/// Prefix of the join table of a many-to-many link type.
pub const LINK_TABLE_PREFIX: &str = "of_lnk_";
/// Suffix of the boolean column that records "this property was set by an edit".
pub const SET_SUFFIX: &str = "__set";

/// Table of an object type, for example `of_obj_gate_stand` for `GateStand`.
pub fn object_table(name: &ObjectTypeName) -> String {
    format!("{OBJECT_TABLE_PREFIX}{}", name.as_str().to_snake_case())
}

/// Join table of a many-to-many link type, for example `of_lnk_gate_stands` for `gateStands`.
pub fn link_table(name: &LinkTypeName) -> String {
    format!("{LINK_TABLE_PREFIX}{}", name.as_str().to_snake_case())
}

/// Column of a property, for example `max_load` for `maxLoad`.
pub fn property_column(name: &PropertyName) -> String {
    name.as_str().to_snake_case()
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
    fn names_are_snake_cased_with_prefixes() {
        assert_eq!(object_table(&object("Gate")), "of_obj_gate");
        assert_eq!(object_table(&object("GateStand")), "of_obj_gate_stand");
        assert_eq!(link_table(&link("gateStands")), "of_lnk_gate_stands");
        assert_eq!(property_column(&property("maxLoad")), "max_load");
        assert_eq!(set_column("max_load"), "max_load__set");
    }
}
