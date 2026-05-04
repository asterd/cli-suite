#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    axt_gitctx::fuzz_parse_status_bytes(data);
});
