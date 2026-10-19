#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrs_missing_is_null() {
        let attrs = Attrs::new();
        assert_eq!(attrs.get("tenantId"), &AttrValue::Null);
    }

    #[test]
    fn attrs_list_rejects_null_element() {
        let mut attrs = Attrs::new();
        let bad = AttrValue::List(vec![Scalar::String("eu".into()), Scalar::Null]);
        assert!(attrs.insert("regions", bad).is_err());
        assert_eq!(attrs.get("regions"), &AttrValue::Null);
    }
}
