# astrogre: ASTrustGen subclass.  Match nodes take (c, n, pos) and return u32
# (0 = fail, 1 = success, 2 = abort: CPS recursion budget exhausted).
require_relative '../../lib/astrustgen'

class AstrogreNodeDef < ASTrustGen::NodeDef
  register_gen_task :free

  class Node < ASTrustGen::NodeDef::Node
    def common_param_count = 3
    def result_type = 'u32'

    # Opaque operands: byte strings are baked as literals, the rest is read from the node.
    def opaque_hash_expr(op, f)
      case op.type
      when 'Bytes' then "hash_bytes(#{f})"
      when 'ClassRef' then "unsafe { (*#{f}).hash }"
      when 'Groups' then "{ let mut h = hash_u64(#{f}.len() as u64); for &g in #{f}.iter() { h = hash_merge(h, hash_u64(g as u64)); } h }"
      when 'ByteSet' then "hash_bytes(unsafe { &*(#{f} as *const [u8; 256]) })"
      else raise "unknown opaque operand type #{op.type}"
      end
    end

    def opaque_specialize_arg(op, f)
      if op.type == 'Bytes'
        "a.push(bytes_lit(#{f}));"
      else
        "a.push(\"u.#{op.name}\".to_string());"
      end
    end

    def opaque_dump_stmt(op, f)
      op.type == 'Bytes' ? "out.push_str(&format!(\"{:?}\", String::from_utf8_lossy(#{f})));" : "out.push_str(\"<#{op.name}>\");"
    end

    def build_free_arm = "        \"#{@name}\" => drop(Box::from_raw(n as *mut NodeOf<#{struct_name}>)),"
  end

  def build_free
    banner + <<~RS
    /// Frees one node allocated by ALLOC_* (patterns own their nodes).
    pub unsafe fn free_node(n: NodePtr) {
        match (*n).head.kind.name {
    #{@nodes.values.map(&:build_free_arm).join("\n")}
            _ => unreachable!(),
        }
    }
    RS
  end
end

ASTrustGen.start(ARGV, node_def_class: AstrogreNodeDef)
