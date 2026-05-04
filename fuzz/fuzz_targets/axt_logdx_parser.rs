#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    axt_logdx::command::fuzz_parse_bytes(data);
});
