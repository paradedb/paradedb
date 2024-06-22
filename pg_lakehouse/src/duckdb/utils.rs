pub fn format_csv(csv_str: &str) -> String {
    let csv_split = csv_str.split(',').collect::<Vec<&str>>();
    match csv_split.len() {
        1 => format!("'{}'", csv_str),
        _ => format!(
            "[{}]",
            csv_split
                .iter()
                .map(|&chunk| format!("'{}'", chunk.trim()))
                .collect::<Vec<String>>()
                .join(", ")
        ),
    }
}
