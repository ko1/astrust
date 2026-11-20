# koruby — ASTrustGen extension.
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

class KorubyNodeDef < ASTrustGen::NodeDef
  class Node < ASTrustGen::NodeDef::Node
    def common_param_count = 3
    def result_type = @ret || 'RV'
    def unwrap_expr(e) = "#{e}?"
    def test_type = "Result<bool, Esc>"
    def test_default(e) = "Ok(#{e}?.truthy())"
    # the method entry cached at this call site when the code is baked
    def spec_target_expr(op) = "{ let u = &(*np).u; crate::vm::spec_callee(&u.cc, u.args.len(), u.flags) }"

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

ASTrustGen.start(ARGV, node_def_class: KorubyNodeDef)
