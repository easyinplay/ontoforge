//! Name tokenizer shared by the naming rules.

/// Split a `camelCase` or `PascalCase` name into lowercase words. Boundaries are
/// lower to upper (`releaseDate`), letter to digit and back (`code01`), and the end
/// of an acronym (`HTTPServer` is `http`, `server`).
pub fn split_camel(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            continue;
        }
        let boundary = !word.is_empty() && {
            let prev = chars[i - 1];
            (prev.is_lowercase() && c.is_uppercase())
                || prev.is_alphabetic() != c.is_alphabetic()
                || (prev.is_uppercase()
                    && c.is_uppercase()
                    && chars.get(i + 1).is_some_and(|next| next.is_lowercase()))
        };
        if boundary {
            words.push(std::mem::take(&mut word));
        }
        word.extend(c.to_lowercase());
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::split_camel;

    #[test]
    fn split_camel_cases() {
        assert_eq!(split_camel("ReleaseDate"), ["release", "date"]);
        assert_eq!(split_camel("code01"), ["code", "01"]);
        assert_eq!(split_camel("HTTPServer"), ["http", "server"]);
        assert_eq!(split_camel("FlightV2"), ["flight", "v", "2"]);
        assert_eq!(split_camel("isCurrent"), ["is", "current"]);
        assert_eq!(split_camel("status"), ["status"]);
        assert!(split_camel("").is_empty());
    }
}
