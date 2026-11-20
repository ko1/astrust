// Built-in classes: boot sequence and module registry.
pub mod kernel;
pub mod object;
pub mod module;
pub mod proc;
pub mod exception;
pub mod numeric_methods;
pub mod string_methods;
pub mod symbol_methods;
pub mod array_methods;
pub mod hash_methods;
pub mod range_methods;
pub mod enumerator;
pub mod sprintf;
pub mod pp;
pub mod io;
pub mod marshal;
pub mod strscan;
pub mod json;
pub mod env;
pub mod extras;
pub mod thread;
pub mod sys;
pub mod file;
pub mod process;
pub mod random;
pub mod binding;
pub mod prelude;
pub mod misc;
pub mod regexp_methods;
pub mod structs;
pub mod time;
pub mod math;
pub mod rational;
pub mod complex;
pub mod enum_native;
pub mod encoding;
pub mod stringio;
pub mod enc_tables;

use crate::object::*;
use crate::symbol::intern_str;
use crate::value::*;
use crate::vm::*;
use crate::fxhash::HashMap;

fn blank_vm() -> Box<Vm> {
    // every Value field starts as NIL; the class fields are filled by boot()
    let mut v: std::mem::MaybeUninit<Vm> = std::mem::MaybeUninit::uninit();
    unsafe {
        let p = v.as_mut_ptr() as *mut u64;
        let nclass = std::mem::offset_of!(Vm, main_obj) / 8;
        for i in 0..nclass { *p.add(i) = NIL.raw(); }
        let vp = v.as_mut_ptr();
        std::ptr::write(&mut (*vp).main_obj, NIL);
        std::ptr::write(&mut (*vp).stdout, NIL);
        std::ptr::write(&mut (*vp).stderr, NIL);
        std::ptr::write(&mut (*vp).stdin, NIL);
        std::ptr::write(&mut (*vp).globals, HashMap::default());
        std::ptr::write(&mut (*vp).global_aliases, HashMap::default());
        std::ptr::write(&mut (*vp).method_serial, 1);
        std::ptr::write(&mut (*vp).const_serial, 1);
        std::ptr::write(&mut (*vp).hierarchy_serial, 1);
        std::ptr::write(&mut (*vp).mcache, HashMap::default());
        std::ptr::write(&mut (*vp).gen_ivars, HashMap::default());
        std::ptr::write(&mut (*vp).node_values, vec![]);
        std::ptr::write(&mut (*vp).crefs, vec![]);
        std::ptr::write(&mut (*vp).top_cref, std::ptr::null());
        std::ptr::write(&mut (*vp).loaded_features, NIL);
        std::ptr::write(&mut (*vp).load_path, NIL);
        std::ptr::write(&mut (*vp).script_name, String::new());
        std::ptr::write(&mut (*vp).at_exit, NIL);
        std::ptr::write(&mut (*vp).object_ids, HashMap::default());
        std::ptr::write(&mut (*vp).next_object_id, 0);
        std::ptr::write(&mut (*vp).sources, vec![]);
        std::ptr::write(&mut (*vp).verbose, FALSE);
        std::ptr::write(&mut (*vp).debug_flag, false);
        std::ptr::write(&mut (*vp).aot_entries, vec![]);
        std::ptr::write(&mut (*vp).aot_collect, false);
        std::ptr::write(&mut (*vp).code_store, None);
        std::ptr::write(&mut (*vp).sd_hits, 0);
        std::ptr::write(&mut (*vp).frozen_lits, HashMap::default());
        std::ptr::write(&mut (*vp).symproc_cache, HashMap::default());
        std::ptr::write(&mut (*vp).data_free_hooks, vec![]);
        std::ptr::write(&mut (*vp).pinned, vec![]);
        std::ptr::write(&mut (*vp).recursive_guard, vec![]);
        std::ptr::write(&mut (*vp).argv, NIL);
        std::ptr::write(&mut (*vp).warn_disabled, false);
        std::ptr::write(&mut (*vp).exit_status, 0);
        std::ptr::write(&mut (*vp).main_file, String::new());
        std::ptr::write(&mut (*vp).dollar_zero, NIL);
        std::ptr::write(&mut (*vp).rand_state, 0x2545F4914F6CDD1D);
        let n = std::ptr::null();
        std::ptr::write(&mut (*vp).ids, MethodIds { bo_eq: n, bo_init: n, c_new: n, m_eqq: n, kernel_hash: n, r_each: n });
        std::ptr::write(&mut (*vp).main_ctx, std::ptr::null_mut());
        std::ptr::write(&mut (*vp).fiber_roots, vec![]);
        Box::new(v.assume_init())
    }
}

/// Create the class hierarchy and every built-in method.
pub fn boot() {
    unsafe { VM = Some(blank_vm()); }
    let vm = vm();
    // BasicObject / Object / Module / Class with hand-made metaclasses
    let bo = alloc_class(NIL, NIL, false, AllocKind::Object);
    let ob = alloc_class(NIL, bo, false, AllocKind::Object);
    let md = alloc_class(NIL, ob, false, AllocKind::Module);
    let cl = alloc_class(NIL, md, false, AllocKind::Class);
    vm.c_basic_object = bo; vm.c_object = ob; vm.c_module = md; vm.c_class = cl;
    for k in [bo, ob, md, cl] { obj(k).klass = cl; }
    let mk = |k: Value, sup: Value| -> Value {
        let m = alloc_class(cl, sup, false, AllocKind::Class);
        rclass(m).is_singleton = true; rclass(m).attached = k; obj(k).klass = m; m
    };
    let mbo = mk(bo, cl);
    let mob = mk(ob, mbo);
    let mmd = mk(md, mob);
    let _mcl = mk(cl, mmd);
    for (k, n) in [(bo, "BasicObject"), (ob, "Object"), (md, "Module"), (cl, "Class")] {
        name_class(k, ob, intern_str(n));
        const_set(ob, intern_str(n), k);
    }
    rclass(ob).subclasses.push(md); rclass(md).subclasses.push(cl); rclass(bo).subclasses.push(ob);
    vm.m_kernel = define_module("Kernel");
    include_module(ob, vm.m_kernel);
    vm.m_comparable = define_module("Comparable");
    vm.m_enumerable = define_module("Enumerable");
    let num = define_class("Numeric", ob);
    include_module(num, vm.m_comparable);
    vm.c_numeric = num;
    vm.c_integer = define_class("Integer", num);
    vm.c_float = define_class("Float", num);
    vm.c_string = define_class("String", ob);
    rclass(vm.c_string).alloc = AllocKind::String;
    // name strings made before String existed have no class yet
    for k in [bo, ob, md, cl, vm.m_kernel, vm.m_comparable, vm.m_enumerable, num, vm.c_integer, vm.c_float, vm.c_string] {
        if let Some(n) = rclass(k).name { obj(n).klass = vm.c_string; }
    }
    include_module(vm.c_string, vm.m_comparable);
    vm.c_symbol = define_class("Symbol", ob);
    include_module(vm.c_symbol, vm.m_comparable);
    vm.c_array = define_class("Array", ob);
    rclass(vm.c_array).alloc = AllocKind::Array;
    include_module(vm.c_array, vm.m_enumerable);
    vm.c_hash = define_class("Hash", ob);
    rclass(vm.c_hash).alloc = AllocKind::Hash;
    include_module(vm.c_hash, vm.m_enumerable);
    vm.c_range = define_class("Range", ob);
    rclass(vm.c_range).alloc = AllocKind::Range;
    include_module(vm.c_range, vm.m_enumerable);
    vm.c_nil = define_class("NilClass", ob);
    vm.c_true = define_class("TrueClass", ob);
    vm.c_false = define_class("FalseClass", ob);
    vm.c_proc = define_class("Proc", ob);
    rclass(vm.c_proc).alloc = AllocKind::Proc;
    vm.c_method = define_class("Method", ob);
    rclass(vm.c_method).alloc = AllocKind::None;
    vm.c_unbound_method = define_class("UnboundMethod", ob);
    rclass(vm.c_unbound_method).alloc = AllocKind::None;
    vm.c_env = new_class(ob);
    vm.c_binding = define_class("Binding", ob);
    rclass(vm.c_binding).alloc = AllocKind::None;
    for k in [vm.c_integer, vm.c_float, vm.c_symbol, vm.c_nil, vm.c_true, vm.c_false] { rclass(k).alloc = AllocKind::None; }

    vm.main_obj = new_object(ob);
    vm.loaded_features = crate::array::ary_new();
    vm.load_path = crate::array::ary_new();
    vm.at_exit = crate::array::ary_new();
    vm.argv = crate::array::ary_new();
    vm.top_cref = new_cref(ob, std::ptr::null());

    exception::init_classes();
    object::init();
    kernel::init();
    module::init();
    proc::init();
    exception::init();
    numeric_methods::init();
    string_methods::init();
    symbol_methods::init();
    array_methods::init(vm.c_array);
    hash_methods::init(vm.c_hash);
    vm.ids.r_each = range_methods::init(vm.c_range);
    let e = define_class("Enumerator", vm.c_object);
    rclass(e).alloc = AllocKind::None;
    let k = enumerator::init(e, vm.c_object, vm.m_enumerable);
    vm.c_enumerator = k.enumerator; vm.c_yielder = k.yielder; vm.c_generator = k.generator;
    vm.c_lazy = k.lazy; vm.c_chain = k.chain; vm.c_arith_seq = k.arith_seq;
    io::init();
    process::init();
    marshal::init();
    random::init();
    binding::init();
    thread::init();
    misc::init();
    prelude::lazy_const(ob, "ENV", "saruby/env");
    prelude::lazy_const(ob, "RbConfig", "saruby/rbconfig");
    encoding::init();
    stringio::init();
    regexp_methods::init();
    vm.c_struct = structs::init(vm.c_object, vm.m_enumerable);
    time::init();
    math::init();
    rational::init();
    complex::init();
    enum_native::init(vm.m_enumerable);
    crate::fiber::init();
    extras::init();
}

/// dup/clone of built-in Data objects.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if rational::is_rational(v) || complex::is_complex(v) { return Ok(Some(v)); }
    if let Some(r) = misc::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = regexp_methods::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = time::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = random::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = enumerator::dup_data(c, v)? { return Ok(Some(r)); }
    Ok(None)
}

/// Allocation of instances of built-in Data classes (and their subclasses).
pub fn alloc_data(c: &mut Ctx, klass: Value) -> Result<Option<Value>, Esc> {
    if let Some(r) = misc::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = time::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = random::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = enumerator::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = regexp_methods::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = file::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = io::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = strscan::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_lock(klass) { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_queue(klass) { return Ok(Some(r)); }
    Ok(None)
}

/// Is klass (or an ancestor) the given built-in class?
pub fn inherits(klass: Value, base: Value) -> bool { klass == base || ancestors(klass).contains(&base) }
