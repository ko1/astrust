# saruby — ASTrustGen extension.
#
# Dispatcher signature: (c: &mut Ctx, n: NodePtr, fp: Fp) -> RV
#   fp: the current frame on the VM value stack, a window of Cell slots (fp[0] = self, fp[1] = env).
#   RV = Result<Value, Esc>; an eager (@child) operand propagates Err with `?`.
#
# Operand types beyond astrustgen's built-ins:
#   Sym       symbol id: per-process, so read from the node at run time; hashed by name
#   Value     immediate literal: fixnum/flonum/nil/true/false baked, symbols read at run time
#   Str       &'static [u8] literal bytes: read from the node; hashed by content
#   IseqRef   &'static Iseq (method/block/class body): read from the node; hashed by its body
#   CiRef     &'static CallInfo: read from the node; hashed by content
require_relative '../../lib/astrustgen'
ASTrustGen.value_type = 'Value'

class SarubyNodeDef < ASTrustGen::NodeDef
  class Node < ASTrustGen::NodeDef::Node
    def common_param_count = 3
    def result_type = @ret || 'RV'
    def unwrap_expr(e) = "#{e}?"
    # cargo feature `nodeprof`: count executions per node kind (interpreter only; SARUBY_NODEPROF=1 prints them)
    def dispatch_prologue = "    #[cfg(feature = \"nodeprof\")] crate::nodeprof::hit(\"#{@name}\");\n"
    def test_type = "Result<bool, Esc>"
    def test_default(e) = "Ok(#{e}?.truthy())"
    # bodies are laid out in one block each (compile.rs finish_iseq)
    def contiguous? = true
    # the method entry cached at this call site when the code is baked
    # what the call site's cache holds when the code is baked (vm.rs SITE_*)
    def bake_value_expr(op)
      case @name
      # 1 + the slot index in the site's cache (0: the site never ran)
      when 'node_ivar', 'node_iasgn', 'node_ivar_arith' then "{ let u = &(*np).u; if u.ic.klass != 0 { u.ic.idx + 1 } else { 0 } }"
      # 1 + the operand kinds the interpreter recorded at an operator site (opt.rs TyFb)
      when /\Anode_opt_/ then "{ 1 + (*np).u.tf.0 }"
      # 1 + the lead parameter count of the block compiled into a yield site
      when 'node_call'
        case op.name
        when 'shared' then "{ let u = &(*np).u; crate::vm::spec_shared(u.recv, u.args.len(), crate::vm::spec_callee(&u.cc, u.args.len(), u.flags)) }"
        # 1 + the inline_builtin.rs table index of the builtin the site's cache holds (0: none)
        when 'bi' then "{ let u = &(*np).u; crate::inline_builtin::site_builtin(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags)) }"
        else "{ let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }"
        end
      when 'node_yield' then "{ let u = &(*np).u; crate::vm::spec_block_np(&u.yc, u.args.len()) }"
      # the builtin iterator the site's cache holds, with the block's parameter count (inline_iter.rs; 0: none)
      when 'node_call_blk'
        if op.name == 'it' then "{ let u = &(*np).u; crate::inline_iter::site_iter(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }"
        else "{ let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }"
        end
      when 'node_call_kw' then "{ let u = &(*np).u; crate::vm::site_kind_kw(&u.cc, u.args.len(), u.ci) }"
      else "{ let u = &(*np).u; crate::vm::site_kind(&u.cc, u.args.len(), u.flags) }"
      end
    end
    # an entry's (me, blk): passed by a call that compiled the entry in, else in c.call_me / c.call_blk
    def spec_in_load_expr(op) = "crate::vm::CallArgs::load(c)"
    def spec_in_store_stmt(var) = "#{var}.store(c);"
    def spec_target_expr(op)
      if @name == %q(node_yield) then "{ let u = &(*np).u; crate::vm::spec_block(&u.yc, u.args.len()) }"
      elsif @name == %q(node_call_blk) && op.name == 'body' then "{ let u = &(*np).u; crate::inline_iter::spec_iter_block(&u.cc, u.args.len(), crate::vm::site_kind(&u.cc, u.args.len(), u.flags), u.blk) }"
      elsif @name == %q(node_call_kw) then "{ let u = &(*np).u; crate::vm::spec_callee_kw(&u.cc, u.args.len(), u.ci) }"
      else "{ let u = &(*np).u; crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }"
      end
    end

    def opaque_hash_expr(op, f)
      case op.type
      when 'Sym' then "hash_bytes(crate::symbol::sym_bytes(#{f}))"
      when 'Value' then "crate::compile::value_hash(#{f})"
      when 'Str' then "hash_bytes(#{f})"
      when 'IseqRef' then "crate::compile::iseq_hash(#{f})"
      when 'CiRef' then "crate::compile::ci_hash(#{f})"
      else "0u64"
      end
    end

    def opaque_specialize_arg(op, f)
      case op.type
      when 'Value'
        "a.push(crate::compile::value_bake(#{f}).unwrap_or_else(|| \"(*np).u.#{op.name}\".to_string()));"
      else
        "a.push(\"(*np).u.#{op.name}\".to_string());"
      end
    end

    def opaque_dump_stmt(op, f)
      case op.type
      when 'Sym' then "out.push(':'); out.push_str(&crate::symbol::sym_str(#{f}));"
      when 'Value' then "out.push_str(&format!(\"{:?}\", #{f}));"
      when 'Str' then "out.push_str(&format!(\"{:?}\", String::from_utf8_lossy(#{f})));"
      else "out.push_str(\"<#{op.name}>\");"
      end
    end
  end

  def head_extra_fields = "    pub line: u32,"
  def head_extra_inits = "line: 0,"
end

ASTrustGen.start(ARGV, node_def_class: SarubyNodeDef)
