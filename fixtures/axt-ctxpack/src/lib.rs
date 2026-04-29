pub fn build_message() -> &'static str {
    "TODO in a string"
}

// TODO: replace temporary fallback
pub fn unwrap_value(value: Option<u32>) -> u32 {
    value.unwrap_or(7)
}

#[cfg(test)]
mod tests {
    #[test]
    fn todo_test() {
        assert_eq!("TODO", "TODO");
    }
}
