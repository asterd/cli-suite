#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools
)]

pub mod cli;
pub mod frontend;
pub mod model;

pub fn fuzz_parse_output(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    for framework in [
        cli::FrameworkArg::Jest,
        cli::FrameworkArg::Vitest,
        cli::FrameworkArg::Pytest,
        cli::FrameworkArg::Cargo,
        cli::FrameworkArg::Go,
        cli::FrameworkArg::Bun,
        cli::FrameworkArg::Deno,
    ] {
        let _events = frontend::parse_output(framework, &text, "");
        let _line_events = frontend::parse_stdout_line(framework, &text);
    }
}
