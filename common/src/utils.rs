pub fn call_to_operator(callsign: &str) -> String {
    let callsign = callsign.trim_end().to_string();
    let parts: Vec<_> = callsign.split("/").collect();
    match parts.len() {
        1 => parts[0].to_string(),
        2 => parts[0].to_string(),
        3 => parts[1].to_string(),
        _ => callsign,
    }
}
