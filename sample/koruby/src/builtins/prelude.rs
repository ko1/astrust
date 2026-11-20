// Ruby-level prelude and lazily-loaded built-in features.
use crate::value::*;
use crate::vm::*;

const CORE: &str = include_str!("../../prelude/core.rb");

pub fn load_prelude(c: &mut Ctx) -> RV {
    crate::builtins::kernel::run_source(c, CORE.as_bytes(), "<internal:prelude>", 1)
}

pub fn has_feature(_name: &str) -> bool { false }
pub fn load_feature(_c: &mut Ctx, _name: &str) -> RV { Ok(NIL) }

/// $LOAD_PATH: -I dirs (already added), KORUBY_LIB, and the script's lib dirs.
pub fn setup_load_path() {
    if let Ok(p) = std::env::var("RUBYLIB") {
        for d in p.split(':') { let s = crate::string::str_new(d.as_bytes()); crate::array::ary_push(vm().load_path, s); }
    }
}
