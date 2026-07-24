#![no_main]

use libfuzzer_sys::fuzz_target;
use weavatrix_graph::{graph6_decode, graphml_decode, topology_from_dot};

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = graph6_decode(text);
        let _ = topology_from_dot(text);
        let _ = graphml_decode(text);
    }
});
