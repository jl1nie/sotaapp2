//! コールサインの分解

/// コールサインを、運用者(本来のコールサイン)と、運用の付記(`/1`、`/P`、`QRP`、前置の `JA` など)に分ける
pub fn split_callsign(call: &str) -> (String, String) {
    let call = call.to_uppercase();

    // Pattern: PREFIX/CALL/SUFFIX (e.g., JA/JL1NIE/P)
    let parts: Vec<&str> = call.split('/').collect();

    match parts.len() {
        3 => {
            // Check if middle part starts with digit
            if parts[1]
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                // Pattern like JL1NIE/7/P
                (parts[0].to_string(), format!("{}/{}", parts[1], parts[2]))
            } else {
                // Pattern like JA/JL1NIE/P
                (parts[1].to_string(), parts[0].to_string())
            }
        }
        2 => {
            let first_char = parts[1].chars().next();
            if first_char.map(|c| c.is_ascii_digit()).unwrap_or(false) {
                // Pattern like JL1NIE/7
                (parts[0].to_string(), parts[1].to_string())
            } else if parts[1].to_uppercase() == "QRP" {
                (parts[0].to_string(), parts[1].to_string())
            } else if parts[1].len() > parts[0].len() {
                // PREFIX/CALL
                (parts[1].to_string(), parts[0].to_string())
            } else {
                // CALL/SUFFIX
                (parts[0].to_string(), parts[1].to_string())
            }
        }
        _ => (call.trim().to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_callsign() {
        assert_eq!(
            split_callsign("JL1NIE"),
            ("JL1NIE".to_string(), String::new())
        );
        assert_eq!(
            split_callsign("JL1NIE/7"),
            ("JL1NIE".to_string(), "7".to_string())
        );
        assert_eq!(
            split_callsign("JA/JL1NIE"),
            ("JL1NIE".to_string(), "JA".to_string())
        );
        assert_eq!(
            split_callsign("JL1NIE/QRP"),
            ("JL1NIE".to_string(), "QRP".to_string())
        );
        assert_eq!(
            split_callsign("JA/JL1NIE/P"),
            ("JL1NIE".to_string(), "JA".to_string())
        );
        assert_eq!(
            split_callsign("JL1NIE/7/P"),
            ("JL1NIE".to_string(), "7/P".to_string())
        );
    }
}
