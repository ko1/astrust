// Boot sequence of the built-in classes, and the Data dispatch of dup / allocate.
// Runs on the main context (lib.rs makes it right after `install`); class payloads that have no
// vm.rs setter are set up by the boot helpers of legacy.rs (docs/safe_heap.md, "Method layer").
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
    ($($nil:ident)*; $($f:ident: $e:expr),* $(,)?) => { Box::new(Vm { $($nil: NIL.into(),)* $($f: $e.into()),* }) };
}

fn blank_vm() -> Box<Vm> {
    // the class fields start as nil; boot() fills them
    let n = std::ptr::null::<MethodEntry>();
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
        node_values: vec![], crefs: vec![], top_cref: std::ptr::null::<Cref>(),
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
        ids: MethodIds { bo_eq: n.into(), bo_init: n.into(), c_new: n.into(), m_eqq: n.into(), kernel_hash: n.into(), r_each: n.into(), io_write: n.into() },
        send_cache: crate::vm::send_cache_new(),
        main_ctx: std::ptr::null_mut::<Ctx>(),
    )
}

fn boot_ary(c: &mut Ctx) -> Value { let k = c.vm().c_array.get(); c.alloc(k, Body::Array(crate::avec::AVec::new())) }

/// Create the class hierarchy and every built-in method.
/// Install a blank VM (before the main context is made).
pub fn install() { crate::heap::boot_vm(blank_vm()) }
/// Create the class hierarchy and every built-in method, on the main context.
pub fn boot(c: &mut Ctx) { boot_classes(c) }

fn boot_classes(c: &mut Ctx) {
    let vmr = c.vm();
    // BasicObject / Object / Module / Class with hand-made metaclasses
    let bo = alloc_class(c, NIL, NIL, false, AllocKind::Object);
    let ob = alloc_class(c, NIL, bo, false, AllocKind::Object);
    let md = alloc_class(c, NIL, ob, false, AllocKind::Module);
    let cl = alloc_class(c, NIL, md, false, AllocKind::Class);
    vmr.c_basic_object.set(bo); vmr.c_object.set(ob); vmr.c_module.set(md); vmr.c_class.set(cl);
    for k in [bo, ob, md, cl] { obj_set_klass(k, cl); }
    let mut mk = |k: Value, sup: Value| -> Value {
        let m = alloc_class(c, cl, sup, false, AllocKind::Class);
        boot_metaclass(c, k, m);
        m
    };
    let mbo = mk(bo, cl);
    let mob = mk(ob, mbo);
    let mmd = mk(md, mob);
    let _mcl = mk(cl, mmd);
    for (k, n) in [(bo, "BasicObject"), (ob, "Object"), (md, "Module"), (cl, "Class")] {
        name_class(c, k, ob, intern_str(n));
        const_set(c, ob, intern_str(n), k);
    }
    boot_subclass(c, ob, md); boot_subclass(c, md, cl); boot_subclass(c, bo, ob);
    vmr.m_kernel.set(define_module(c, "Kernel"));
    include_module(c, ob, vmr.m_kernel.get());
    vmr.m_comparable.set(define_module(c, "Comparable"));
    vmr.m_enumerable.set(define_module(c, "Enumerable"));
    let num = define_class(c, "Numeric", ob);
    include_module(c, num, vmr.m_comparable.get());
    vmr.c_numeric.set(num);
    vmr.c_integer.set(define_class(c, "Integer", num));
    vmr.c_float.set(define_class(c, "Float", num));
    vmr.c_string.set(define_class(c, "String", ob));
    class_set_alloc(c, vmr.c_string.get(), AllocKind::String);
    // name strings made before String existed have no class yet
    for k in [bo, ob, md, cl, vmr.m_kernel.get(), vmr.m_comparable.get(), vmr.m_enumerable.get(), num, vmr.c_integer.get(), vmr.c_float.get(), vmr.c_string.get()] {
        if let Some(n) = class_name_sym(c, k) { obj_set_klass(n, vmr.c_string.get()); }
    }
    include_module(c, vmr.c_string.get(), vmr.m_comparable.get());
    vmr.c_symbol.set(define_class(c, "Symbol", ob));
    include_module(c, vmr.c_symbol.get(), vmr.m_comparable.get());
    vmr.c_array.set(define_class(c, "Array", ob));
    class_set_alloc(c, vmr.c_array.get(), AllocKind::Array);
    include_module(c, vmr.c_array.get(), vmr.m_enumerable.get());
    vmr.c_hash.set(define_class(c, "Hash", ob));
    class_set_alloc(c, vmr.c_hash.get(), AllocKind::Hash);
    include_module(c, vmr.c_hash.get(), vmr.m_enumerable.get());
    vmr.c_range.set(define_class(c, "Range", ob));
    class_set_alloc(c, vmr.c_range.get(), AllocKind::Range);
    include_module(c, vmr.c_range.get(), vmr.m_enumerable.get());
    vmr.c_nil.set(define_class(c, "NilClass", ob));
    vmr.c_true.set(define_class(c, "TrueClass", ob));
    vmr.c_false.set(define_class(c, "FalseClass", ob));
    vmr.c_proc.set(define_class(c, "Proc", ob));
    class_set_alloc(c, vmr.c_proc.get(), AllocKind::Proc);
    vmr.c_method.set(define_class(c, "Method", ob));
    class_set_alloc(c, vmr.c_method.get(), AllocKind::None);
    vmr.c_unbound_method.set(define_class(c, "UnboundMethod", ob));
    class_set_alloc(c, vmr.c_unbound_method.get(), AllocKind::None);
    vmr.c_env.set(new_class(c, ob));
    vmr.c_binding.set(define_class(c, "Binding", ob));
    class_set_alloc(c, vmr.c_binding.get(), AllocKind::None);
    for k in [vmr.c_integer.get(), vmr.c_float.get(), vmr.c_symbol.get(), vmr.c_nil.get(), vmr.c_true.get(), vmr.c_false.get()] { class_set_alloc(c, k, AllocKind::None); }

    vmr.main_obj.set(new_object(c, ob));
    vmr.loaded_features.set(boot_ary(c));
    // the features CRuby 4.0 provides at boot
    for f in ["enumerator.so", "thread.rb", "fiber.so", "rational.so", "complex.so", "pathname.so", "ruby2_keywords.rb", "set.rb"] {
        let s = c.str_new(f.as_bytes());
        c.ary_push(vmr.loaded_features.get(), s);
    }
    vmr.load_path.set(boot_ary(c));
    kernel::init_load_path(c, vmr.load_path.get());
    vmr.at_exit.set(boot_ary(c));
    vmr.argv.set(boot_ary(c));
    vmr.top_cref.set(new_cref(ob, std::ptr::null()));

    exception::init_classes(c);
    let ids = object::init(c);
    vmr.ids.bo_init.set(ids.bo_init); vmr.ids.bo_eq.set(ids.bo_eq); vmr.ids.kernel_hash.set(ids.kernel_hash); vmr.ids.m_eqq.set(ids.m_eqq); vmr.ids.c_new.set(ids.c_new);
    kernel::init(c);
    boot_autoload(c, vmr.c_object.get(), &["ARGF"], "saruby/argf");
    proc::init(c);
    exception::init(c);
    numeric_methods::init(c, vmr.c_numeric.get(), vmr.c_integer.get(), vmr.c_float.get());
    for (nm, v) in numeric_methods::FLOAT_CONSTS {
        let f = match Value::flonum(v) { Some(f) => f, None => { let f = c.alloc(vmr.c_float.get(), Body::Float(v)); obj_set_flags(f, FL_FROZEN); f } };
        const_set(c, vmr.c_float.get(), crate::symbol::intern(nm.as_bytes()), f);
    }
    string_methods::init(c, vmr.c_string.get());
    symbol_methods::init(c, vmr.c_symbol.get());
    array_methods::init(c, vmr.c_array.get());
    hash_methods::init(c, vmr.c_hash.get());
    vmr.ids.r_each.set(range_methods::init(c, vmr.c_range.get()));
    let e = define_class(c, "Enumerator", vmr.c_object.get());
    class_set_alloc(c, e, AllocKind::None);
    let k = enumerator::init(c, e, vmr.c_object.get(), vmr.m_enumerable.get());
    vmr.c_enumerator.set(k.enumerator); vmr.c_yielder.set(k.yielder); vmr.c_generator.set(k.generator);
    vmr.c_lazy.set(k.lazy); vmr.c_chain.set(k.chain); vmr.c_arith_seq.set(k.arith_seq);
    io::init(c);
    lazy_const(c, vmr.c_io.get(), "Buffer", "saruby/io_buffer");
    process::init(c);
    vmr.m_marshal.set(marshal::init(c));
    let (r, d) = random::init(c);
    vmr.c_random.set(r); vmr.default_random.set(d);
    binding::init(c);
    thread::init(c);
    lazy_const(c, ob, "Ractor", "saruby/ractor");
    crate::builtins::error_highlight::boot(c);
    crate::builtins::did_you_mean::boot(c);
    misc::init(c);
    lazy_const(c, ob, "ENV", "saruby/env");
    lazy_const(c, ob, "RbConfig", "saruby/rbconfig");
    lazy_const(c, ob, "Pathname", "saruby/pathname");
    encoding::init(c);
    lazy_const(c, vmr.c_encoding.get(), "Converter", "saruby/econv");
    tracepoint::init(c);
    stringio::init(c);
    let (r, m) = regexp_methods::init(c);
    vmr.c_regexp.set(r); vmr.c_match_data.set(m);
    vmr.c_struct.set(structs::init(c, vmr.c_object.get(), vmr.m_enumerable.get()));
    vmr.c_time.set(time::init(c));
    math::init(c, vmr.m_math.get());
    vmr.c_rational.set(rational::init(c));
    vmr.c_complex.set(complex::init(c));
    enum_native::init(c, vmr.m_enumerable.get());
    lazy::init(c, vmr.c_lazy.get(), vmr.m_enumerable.get());
    enum_native::init_comparable(c, vmr.m_comparable.get());
    crate::fiber::init(c);
    fiber_methods::init(c);
    extras::init(c);
}

/// Constant `name` under `owner` whose native definition (`feature`) runs on first reference; boot time, no context.
pub fn lazy_const(c: &mut Ctx, owner: Value, name: &str, feature: &str) { boot_autoload(c, owner, &[name], feature); }
/// lazy_const for several names of one feature.
pub fn lazy_consts(c: &mut Ctx, owner: Value, names: &[&str], feature: &str) { boot_autoload(c, owner, names, feature); }

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
    if let Some(r) = io_buffer::dup_data(c, v)? { return Ok(Some(r)); }
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
    if let Some(r) = io_buffer::alloc_data(c, klass)? { return Ok(Some(r)); }
    Ok(None)
}

/// Is klass (or an ancestor) the given built-in class?
pub fn inherits(c: &Ctx, klass: Value, base: Value) -> bool { klass == base || ancestors(c, klass).contains(&base) }
