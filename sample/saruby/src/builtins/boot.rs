// Boot sequence of the built-in classes, and the Data dispatch of dup / allocate.
// No context exists during boot: the Vm is passed by `heap::boot_vm`; class payloads that have
// no vm.rs setter are set up by the boot helpers of legacy.rs (docs/safe_heap.md, "Method layer").
#![forbid(unsafe_code)]
#[allow(unused_imports)]
use crate::heap::no_legacy::*;   // the old accessors cannot be named here
#[allow(unused_imports)]
use crate::heap::no_legacy_hn::*;
#[allow(unused_imports)]
use crate::heap::no_legacy_str::*;
use super::*;
use crate::fxhash::HashMap;
use crate::heap::{obj_set_flags, obj_set_klass};
use crate::legacy::{boot_autoload, boot_metaclass, boot_subclass};
use crate::object::*;
use crate::symbol::intern_str;
use crate::value::*;
use crate::vm::*;

macro_rules! blank_vm {
    ($($nil:ident)*; $($f:ident: $e:expr),* $(,)?) => { Box::new(Vm { $($nil: NIL,)* $($f: $e),* }) };
}

fn blank_vm() -> Box<Vm> {
    // the class fields start as nil; boot() fills them
    let n = std::ptr::null();
    blank_vm!(
        c_basic_object c_object c_module c_class m_kernel m_comparable m_enumerable c_nil c_true c_false c_numeric
        c_integer c_float c_string c_symbol c_array c_hash c_range c_proc c_method c_unbound_method c_env c_exception
        c_standard_error c_runtime_error c_argument_error c_type_error c_name_error c_no_method_error
        c_zero_div_error c_index_error c_key_error c_range_error c_stop_iteration c_frozen_error c_local_jump_error
        c_float_domain_error c_io_error c_eof_error c_system_exit c_script_error c_load_error c_not_impl_error
        c_syntax_error c_system_stack_error c_no_matching_pattern_error c_no_matching_pattern_key_error c_fiber_error
        c_uncaught_throw_error c_regexp_error c_encoding_error c_math_domain_error c_system_call_error c_interrupt
        c_signal_exception c_security_error c_thread_error c_closed_queue_error c_no_memory_error c_compat_error
        c_invalid_byte_seq_error c_undef_conv_error c_converter_not_found_error c_struct c_regexp c_match_data c_time
        c_io c_file c_enumerator c_lazy c_yielder c_generator c_random c_encoding c_rational c_complex c_binding
        c_data c_set c_fiber c_thread c_mutex c_queue c_arith_seq c_chain c_object_space_weak_map c_string_io c_dir
        c_stat c_monitor m_errno m_math m_process m_gc m_object_space m_marshal m_signal m_warning m_file_test
        c_string_scanner c_string_scanner_error c_econv c_tracepoint trace_list c_location default_random
        main_obj stdout stderr stdin loaded_features load_path at_exit argv dollar_zero;
        verbose: FALSE,
        globals: HashMap::default(), global_aliases: HashMap::default(),
        const_serial: 1, hierarchy_serial: 1,
        mcache: HashMap::default(), gen_ivars: HashMap::default(),
        node_values: vec![], crefs: vec![], top_cref: std::ptr::null(),
        script_name: String::new(),
        object_ids: HashMap::default(), next_object_id: 0,
        sources: vec![], debug_flag: false, raise_hooks: false,
        aot_entries: vec![], aot_collect: false, code_store: None, sd_hits: 0,
        frozen_lits: HashMap::default(), symproc_cache: HashMap::default(), sym_names: HashMap::default(),
        alias_names: HashMap::default(),
        data_free_hooks: vec![], pinned: vec![], enc_objs: vec![], recursive_guard: vec![],
        warn_disabled: false, exit_status: 0, main_file: String::new(),
        rand_state: 0x2545F4914F6CDD1D,
        fiber_roots: vec![],
        blop: None,
        ids: MethodIds { bo_eq: n, bo_init: n, c_new: n, m_eqq: n, kernel_hash: n, r_each: n },
        send_cache: crate::vm::send_cache_new(),
        main_ctx: std::ptr::null_mut(),
    )
}

fn boot_ary(vmr: &Vm) -> Value { crate::gc::alloc(vmr.c_array, Body::Array(crate::avec::AVec::new())) }

/// Create the class hierarchy and every built-in method.
pub fn boot() { crate::heap::boot_vm(blank_vm(), boot_classes) }

fn boot_classes(vmr: &mut Vm) {
    // BasicObject / Object / Module / Class with hand-made metaclasses
    let bo = alloc_class(NIL, NIL, false, AllocKind::Object);
    let ob = alloc_class(NIL, bo, false, AllocKind::Object);
    let md = alloc_class(NIL, ob, false, AllocKind::Module);
    let cl = alloc_class(NIL, md, false, AllocKind::Class);
    vmr.c_basic_object = bo; vmr.c_object = ob; vmr.c_module = md; vmr.c_class = cl;
    for k in [bo, ob, md, cl] { obj_set_klass(k, cl); }
    let mk = |k: Value, sup: Value| -> Value {
        let m = alloc_class(cl, sup, false, AllocKind::Class);
        boot_metaclass(k, m);
        m
    };
    let mbo = mk(bo, cl);
    let mob = mk(ob, mbo);
    let mmd = mk(md, mob);
    let _mcl = mk(cl, mmd);
    for (k, n) in [(bo, "BasicObject"), (ob, "Object"), (md, "Module"), (cl, "Class")] {
        name_class(k, ob, intern_str(n));
        const_set(ob, intern_str(n), k);
    }
    boot_subclass(ob, md); boot_subclass(md, cl); boot_subclass(bo, ob);
    vmr.m_kernel = define_module("Kernel");
    include_module(ob, vmr.m_kernel);
    vmr.m_comparable = define_module("Comparable");
    vmr.m_enumerable = define_module("Enumerable");
    let num = define_class("Numeric", ob);
    include_module(num, vmr.m_comparable);
    vmr.c_numeric = num;
    vmr.c_integer = define_class("Integer", num);
    vmr.c_float = define_class("Float", num);
    vmr.c_string = define_class("String", ob);
    class_set_alloc(vmr.c_string, AllocKind::String);
    // name strings made before String existed have no class yet
    for k in [bo, ob, md, cl, vmr.m_kernel, vmr.m_comparable, vmr.m_enumerable, num, vmr.c_integer, vmr.c_float, vmr.c_string] {
        if let Some(n) = class_name_sym(k) { obj_set_klass(n, vmr.c_string); }
    }
    include_module(vmr.c_string, vmr.m_comparable);
    vmr.c_symbol = define_class("Symbol", ob);
    include_module(vmr.c_symbol, vmr.m_comparable);
    vmr.c_array = define_class("Array", ob);
    class_set_alloc(vmr.c_array, AllocKind::Array);
    include_module(vmr.c_array, vmr.m_enumerable);
    vmr.c_hash = define_class("Hash", ob);
    class_set_alloc(vmr.c_hash, AllocKind::Hash);
    include_module(vmr.c_hash, vmr.m_enumerable);
    vmr.c_range = define_class("Range", ob);
    class_set_alloc(vmr.c_range, AllocKind::Range);
    include_module(vmr.c_range, vmr.m_enumerable);
    vmr.c_nil = define_class("NilClass", ob);
    vmr.c_true = define_class("TrueClass", ob);
    vmr.c_false = define_class("FalseClass", ob);
    vmr.c_proc = define_class("Proc", ob);
    class_set_alloc(vmr.c_proc, AllocKind::Proc);
    vmr.c_method = define_class("Method", ob);
    class_set_alloc(vmr.c_method, AllocKind::None);
    vmr.c_unbound_method = define_class("UnboundMethod", ob);
    class_set_alloc(vmr.c_unbound_method, AllocKind::None);
    vmr.c_env = new_class(ob);
    vmr.c_binding = define_class("Binding", ob);
    class_set_alloc(vmr.c_binding, AllocKind::None);
    for k in [vmr.c_integer, vmr.c_float, vmr.c_symbol, vmr.c_nil, vmr.c_true, vmr.c_false] { class_set_alloc(k, AllocKind::None); }

    vmr.main_obj = new_object(ob);
    vmr.loaded_features = boot_ary(vmr);
    vmr.load_path = boot_ary(vmr);
    kernel::init_load_path(vmr.load_path);
    vmr.at_exit = boot_ary(vmr);
    vmr.argv = boot_ary(vmr);
    vmr.top_cref = new_cref(ob, std::ptr::null());

    exception::init_classes(vmr);
    let ids = object::init(vmr);
    vmr.ids.bo_init = ids.bo_init; vmr.ids.bo_eq = ids.bo_eq; vmr.ids.kernel_hash = ids.kernel_hash; vmr.ids.m_eqq = ids.m_eqq; vmr.ids.c_new = ids.c_new;
    kernel::init(vmr);
    boot_autoload(vmr.c_object, &["ARGF"], "saruby/argf");
    proc::init(vmr);
    exception::init(vmr);
    numeric_methods::init(vmr.c_numeric, vmr.c_integer, vmr.c_float);
    for (nm, v) in numeric_methods::FLOAT_CONSTS {
        let f = match Value::flonum(v) { Some(f) => f, None => { let f = crate::gc::alloc(vmr.c_float, Body::Float(v)); obj_set_flags(f, FL_FROZEN); f } };
        const_set(vmr.c_float, crate::symbol::intern(nm.as_bytes()), f);
    }
    string_methods::init(vmr.c_string);
    symbol_methods::init(vmr.c_symbol);
    array_methods::init(vmr.c_array);
    hash_methods::init(vmr.c_hash);
    vmr.ids.r_each = range_methods::init(vmr.c_range);
    let e = define_class("Enumerator", vmr.c_object);
    class_set_alloc(e, AllocKind::None);
    let k = enumerator::init(e, vmr.c_object, vmr.m_enumerable);
    vmr.c_enumerator = k.enumerator; vmr.c_yielder = k.yielder; vmr.c_generator = k.generator;
    vmr.c_lazy = k.lazy; vmr.c_chain = k.chain; vmr.c_arith_seq = k.arith_seq;
    io::init(vmr);
    process::init(vmr);
    vmr.m_marshal = marshal::init();
    (vmr.c_random, vmr.default_random) = random::init(vmr);
    binding::init(vmr);
    thread::init(vmr);
    misc::init(vmr);
    lazy_const(ob, "ENV", "saruby/env");
    lazy_const(ob, "RbConfig", "saruby/rbconfig");
    lazy_const(ob, "Pathname", "saruby/pathname");
    encoding::init(vmr);
    lazy_const(vmr.c_encoding, "Converter", "saruby/econv");
    tracepoint::init(vmr);
    stringio::init(vmr);
    (vmr.c_regexp, vmr.c_match_data) = regexp_methods::init(vmr);
    vmr.c_struct = structs::init(vmr.c_object, vmr.m_enumerable);
    vmr.c_time = time::init(vmr);
    math::init(vmr.m_math);
    vmr.c_rational = rational::init(vmr);
    vmr.c_complex = complex::init(vmr);
    enum_native::init(vmr.m_enumerable);
    crate::fiber::init(vmr);
    fiber_methods::init(vmr);
    extras::init(vmr);
}

/// Constant `name` under `owner` whose native definition (`feature`) runs on first reference; boot time, no context.
pub fn lazy_const(owner: Value, name: &str, feature: &str) { boot_autoload(owner, &[name], feature); }
/// lazy_const for several names of one feature.
pub fn lazy_consts(owner: Value, names: &[&str], feature: &str) { boot_autoload(owner, names, feature); }

/// dup/clone of built-in Data objects.
pub fn dup_data(c: &mut Ctx, v: Value) -> Result<Option<Value>, Esc> {
    if rational::is_rational(c, v) || complex::is_complex(c, v) { return Ok(Some(v)); }
    if let Some(r) = misc::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = regexp_methods::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = time::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = random::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = enumerator::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = io::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = binding::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = digest::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = stringio::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = strscan::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = zlib::dup_data(c, v)? { return Ok(Some(r)); }
    if let Some(r) = json_generator::dup_data(c, v)? { return Ok(Some(r)); }
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
    if let Some(r) = psych::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = socket::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = transcode::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = tracepoint::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_lock(c, klass) { return Ok(Some(r)); }
    if let Some(r) = thread::alloc_queue(c, klass) { return Ok(Some(r)); }
    if let Some(r) = digest::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = stringio::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = zlib::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = json_parser::alloc_data(c, klass)? { return Ok(Some(r)); }
    if let Some(r) = json_generator::alloc_data(c, klass)? { return Ok(Some(r)); }
    Ok(None)
}

/// Is klass (or an ancestor) the given built-in class?
pub fn inherits(klass: Value, base: Value) -> bool { klass == base || ancestors(klass).contains(&base) }
