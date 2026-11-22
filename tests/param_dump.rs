//! Prints every parameter (id, name, default, range) for the docs.
use nice_plug::prelude::*;
use throw_back::TbParams;

#[test]
fn dump_params() {
    let p = TbParams::default();
    for (id, ptr, group) in p.param_map() {
        unsafe {
            let d = ptr.default_normalized_value();
            println!("PARAM|{id}|{group}|{}|{}|{}|{}", ptr.name(), ptr.normalized_value_to_string(d, true), ptr.normalized_value_to_string(0.0, true), ptr.normalized_value_to_string(1.0, true));
        }
    }
}
