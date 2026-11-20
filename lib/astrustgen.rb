require 'optparse'

# ASTrustGen — the Rust counterpart of ASTro's ASTroGen (astro/lib/astrogen.rb).
#
# Reads a node.def whose NODE_DEF bodies are Rust and generates, per node kind:
#   EVAL_<name>       the evaluator (user body), generic over its children's dispatchers
#   DISPATCH_<name>   the interpreter dispatcher (children dispatched through the node head)
#   ALLOC_<name>      the allocator
#   HASH_<name>       the structural (Merkle) hash
#   SPECIALIZE_<name> the partial evaluator: writes Rust source for a specialized dispatcher
#   DUMP_<name>       an S-expression dumper
#   REPLACE_<name>    child replacement
#
# Where ASTroGen passes a lazy child as (NODE *, dispatcher function pointer) and relies
# on the C compiler to constant-fold the pointer in specialized code, ASTrustGen makes
# the evaluator generic over a `Disp` type per child: the interpreter instantiates it
# with `Dyn` (calls through n.head.dispatcher), a specialized dispatcher with the
# child's own zero-sized SD type, so the static call (and inlining) is guaranteed by
# monomorphization rather than hoped for.
#
# node.def syntax:
#
#   <Rust lines, copied to node_eval.rs>
#   NODE_DEF [@noinline] [@always_inline] [@canonical=BASE]
#   node_name(c: &mut Ctx, n: *mut Node, <common...>, op: TYPE, ...) -> RESULT?
#   {
#       <Rust body>
#   }
#
# Operand forms:
#   x: NODE           lazy child; evaluate with EVAL_ARG!(c, x) (optionally EVAL_ARG!(c, x, <common args...>))
#   x@child: VALUE    eager child: evaluated by the generated glue, the body receives the value
#   xs: NODES         a list of lazy children: EVAL_LEN!(xs), EVAL_NTH!(c, xs, i)
#   x@ref: T          T stored inline in the node; the body receives `&mut T` (caches etc.); not hashed
#   x: u32|i32|u64|i64|usize|bool|f64   scalars: hashed and baked into specialized code as literals
#   x: <other T>      opaque: read from the node at run time; hashed through the embedder's hook
module ASTrustGen
  class << self; attr_accessor :value_type; end
  self.value_type = 'Value'

  class NodeDef
    GenTask = Struct.new(:name)
    def self.gen_tasks
      @gen_tasks ||= (superclass.respond_to?(:gen_tasks) ? superclass.gen_tasks.dup : [])
    end
    def self.register_gen_task(name)
      gen_tasks.reject! { |t| t.name == name }
      gen_tasks << GenTask.new(name)
    end
    %i[eval dispatch hash specialize dump replace].each { register_gen_task it }

    def initialize file, opt
      @file = file
      @opt = opt
      @nodes = {}
    end

    class Node
      class Operand
        SCALARS = {
          'u32' => '{}u32', 'i32' => '{}i32', 'u64' => '{}u64', 'i64' => '{}i64',
          'usize' => '{}usize', 'bool' => '{}', 'u8' => '{}u8', 'u16' => '{}u16',
        }
        attr_reader :name, :type
        attr_accessor :owner

        def initialize name, type
          @ref = name.end_with?('@ref')
          @child = name.end_with?('@child')
          @name = name.sub(/@(ref|child)$/, '')
          @type = type.strip
        end

        def ref? = @ref
        def child? = @child
        def node? = !@ref && !@child && @type == 'NODE'
        def nodes? = !@ref && @type == 'NODES'
        def any_node? = child? || node? || nodes?
        def scalar? = !ref? && SCALARS.key?(@type)
        def f64? = !ref? && @type == 'f64'

        # Rust type of the field in the node's operand struct.
        def storage_type
          return 'NodePtr' if child? || node?
          return 'NodeList' if nodes?
          @type
        end

        # Parameter(s) of EVAL_<name>.
        def eval_param
          if ref? then "#{@name}: &mut #{@type}"
          elsif child? then "#{@name}: #{@type}"
          elsif node? then "#{@name}: NodePtr"
          elsif nodes? then "#{@name}: NodeList"
          else "#{@name}: #{@type}"
          end
        end

        def generic_param
          if node? then "D_#{@name}: Disp"
          elsif nodes? then "D_#{@name}: DispList"
          end
        end

        def field(u = 'u') = "#{u}.#{@name}"

        # Rust expression (u64) hashing this operand; `u` names the operand struct.
        def hash_expr(u = 'u')
          f = field(u)
          if child? || node? then "hash_node(#{f})"
          elsif nodes? then "hash_node_list(#{f})"
          elsif ref? then nil
          elsif scalar? then "hash_u64(#{f} as u64)"
          elsif f64? then "hash_u64(#{f}.to_bits())"
          else @owner.opaque_hash_expr(self, f)
          end
        end

        # Rust statement(s) in SPECIALIZE_ appending this operand's EVAL argument
        # to `a` (a Vec<String>) — the baked literal, or a read from the node.
        def specialize_arg
          f = "(*np).u.#{@name}"
          if ref? then "a.push(\"&mut (*np).u.#{@name}\".to_string());"
          elsif child? then "a.push(\"v_#{@name}\".to_string());"
          elsif node? || nodes? then "a.push(\"(*np).u.#{@name}\".to_string());"
          elsif scalar? then "a.push(format!(\"#{SCALARS[@type]}\", #{f}));"
          elsif f64? then "a.push(format!(\"f64::from_bits({}u64)\", #{f}.to_bits()));"
          else @owner.opaque_specialize_arg(self, f)
          end
        end

        def dump_stmt
          f = "(*np).u.#{@name}"
          if child? || node? then "dump_node(out, #{f});"
          elsif nodes? then "out.push('['); for (i, x) in #{f}.iter().enumerate() { if i > 0 { out.push(' '); } dump_node(out, *x); } out.push(']');"
          elsif ref? then "out.push_str(\"<#{@name}>\");"
          elsif scalar? || f64? then "out.push_str(&format!(\"{:?}\", #{f}));"
          else @owner.opaque_dump_stmt(self, f)
          end
        end
      end

      attr_reader :name, :operands, :prefix_params

      def initialize name, params_str, ret, option
        @name = name
        @ret = ret
        @option = option&.split(/\s+/) || []
        parse_operands(params_str)
      end

      # ---- embedder hooks ----------------------------------------------------
      # Common parameters shared by every dispatcher (default `c, n`).
      def common_param_count = 2
      def result_type = @ret || ASTrustGen.value_type
      # Extract the value from a child dispatcher's result inside DISPATCH/SD glue
      # (e.g. `e?` when the result is a Result).
      def unwrap_expr(e) = e
      # Opaque (non-scalar, non-node) operands: hash / specialize / dump.
      def opaque_hash_expr(op, f) = "0u64"
      def opaque_specialize_arg(op, f) = "a.push(\"(*np).u.#{op.name}\".to_string());"
      def opaque_dump_stmt(op, f) = "out.push_str(\"<#{op.name}>\");"
      def dispatch_prologue = ""

      def no_inline? = @option.include?('@noinline')
      def always_inline? = @option.include?('@always_inline')
      def canonical_name
        o = @option.find { it.start_with?('@canonical=') }
        o ? o.sub('@canonical=', '') : @name
      end

      # ------------------------------------------------------------------------
      def split_params(str)
        depth = 0; cur = +""; out = []
        str.each_char do |ch|
          case ch
          when '<', '(', '[' then depth += 1
          when '>', ')', ']' then depth -= 1
          end
          if ch == ',' && depth == 0 then out << cur; cur = +"" else cur << ch end
        end
        out << cur unless cur.strip.empty?
        out.map(&:strip)
      end

      def parse_operands str
        ps = split_params(str)
        @prefix_params = ps.shift(common_param_count)
        @operands = ps.map do |p|
          raise "ill-formed operand: #{p}" unless /\A([a-zA-Z_][a-zA-Z0-9_]*(?:@ref|@child)?)\s*:\s*(.+)\z/ =~ p
          op = self.class::Operand.new($1, $2)
          op.owner = self
          op
        end
      end

      def prefix_names = @prefix_params.map { it.split(':').first.strip }
      def extra_call_args = prefix_names.drop(2)
      def struct_name = "#{@name}_struct"

      def parse_body lines
        head = lines.shift.chomp
        raise "illformed body header: #{head.inspect} (#{@name})" unless head == "{"
        @body = +""
        loop do
          line = lines.shift or raise "unterminated body: #{@name}"
          line = line.chomp
          break if line == "}"
          @body << line << "\n"
        end
      end

      # EVAL_ARG!(c, x) / EVAL_ARG!(c, x, extra...) / EVAL_NTH!(c, xs, i) / EVAL_LEN!(xs)
      def substitute_body(body)
        extra = extra_call_args
        body.gsub(/EVAL_ARG!\(\s*(\w+)\s*,\s*(\w+)\s*(,[^()]*(?:\([^()]*\)[^()]*)*)?\)/) {
          c, x, rest = $1, $2, $3
          args = rest ? rest.sub(/\A,/, '').strip : extra.join(', ')
          "<D_#{x} as Disp>::call(#{c}, #{x}#{args.empty? ? '' : ", #{args}"})"
        }.gsub(/EVAL_NTH!\(\s*(\w+)\s*,\s*(\w+)\s*,\s*([^,()]+(?:\([^()]*\))?)\s*(,[^()]*)?\)/) {
          c, xs, i, rest = $1, $2, $3, $4
          args = rest ? rest.sub(/\A,/, '').strip : extra.join(', ')
          "<D_#{xs} as DispList>::nth(#{c}, #{xs}, #{i}#{args.empty? ? '' : ", #{args}"})"
        }.gsub(/EVAL_LEN!\(\s*(\w+)\s*\)/) { "<D_#{$1} as DispList>::len(#{$1})" }
      end

      def generics
        g = @operands.filter_map(&:generic_param)
        g.empty? ? "" : "<#{g.join(', ')}>"
      end

      def build_eval
        params = (@prefix_params + @operands.map(&:eval_param)).join(', ')
        <<~RS
        #[inline(always)]
        #[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
        pub fn EVAL_#{@name}#{generics}(#{params}) -> #{result_type} {
        #{substitute_body(@body)}}
        RS
      end

      def build_struct
        fields = @operands.map { "    pub #{it.name}: #{it.storage_type}," }
        <<~RS
        #[allow(non_camel_case_types)]
        #[repr(C)]
        pub struct #{struct_name} {
        #{fields.join("\n")}
        }
        RS
      end

      def alloc_params = @operands.reject(&:ref?).map { "#{it.name}: #{it.storage_type}" }.join(', ')

      def build_alloc
        inits = @operands.map { it.ref? ? "#{it.name}: Default::default()" : it.name }
        <<~RS
        #[allow(non_snake_case)]
        pub fn ALLOC_#{@name}(#{alloc_params}) -> NodePtr {
            node_allocate(&KIND_#{@name}, #{struct_name} { #{inits.join(', ')} })
        }
        RS
      end

      def build_kind
        <<~RS
        #[allow(non_upper_case_globals)]
        pub static KIND_#{@name}: NodeKind = NodeKind {
            name: "#{@name}",
            default_dispatcher: DISPATCH_#{@name},
            hash_func: HASH_#{@name},
            specializer: SPECIALIZE_#{@name},
            dumper: DUMP_#{@name},
            replacer: REPLACE_#{@name},
            no_inline: #{no_inline?},
        };
        RS
      end

      # Argument list for EVAL in DISPATCH and SD bodies.
      def eval_call_args
        args = prefix_names.dup
        @operands.each do |op|
          if op.ref? then args << "&mut (*np).u.#{op.name}"
          elsif op.child? then args << "v_#{op.name}"
          else args << "(*np).u.#{op.name}"
          end
        end
        args.join(', ')
      end

      def turbofish(types) = types.empty? ? "" : "::<#{types.join(', ')}>"

      def child_eval_stmts(disp_of)
        @operands.select(&:child?).map do |op|
          "    let v_#{op.name} = #{unwrap_expr("<#{disp_of.(op)} as Disp>::call(#{(prefix_names.take(1) + ["(*np).u.#{op.name}"] + extra_call_args).join(', ')})")};"
        end
      end

      def build_dispatch
        ty = @operands.filter_map { it.generic_param ? (it.nodes? ? "DynList" : "Dyn") : nil }
        <<~RS
        #[allow(non_snake_case, unused_variables, unused_unsafe)]
        pub fn DISPATCH_#{@name}(#{@prefix_params.join(', ')}) -> #{result_type} {
        #{dispatch_prologue}    let np = #{prefix_names[1]} as *mut NodeOf<#{struct_name}>;
            unsafe {
        #{child_eval_stmts(->(_) { "Dyn" }).join("\n")}
            EVAL_#{@name}#{turbofish(ty)}(#{eval_call_args})
            }
        }
        RS
      end

      def build_hash
        hs = @operands.filter_map(&:hash_expr).map { "    h = hash_merge(h, #{it});" }
        <<~RS
        #[allow(non_snake_case, unused_unsafe, unused_variables, unused_mut)]
        fn HASH_#{@name}(n: NodePtr) -> u64 {
            let u = unsafe { &(*(n as *mut NodeOf<#{struct_name}>)).u };
            let mut h = hash_str("#{canonical_name}");
        #{hs.join("\n")}
            h
        }
        RS
      end

      def build_dump
        parts = @operands.map { "    out.push(' '); #{it.dump_stmt}" }
        <<~RS
        #[allow(non_snake_case, unused_unsafe, unused_variables)]
        fn DUMP_#{@name}(out: &mut String, n: NodePtr) {
            let np = n as *mut NodeOf<#{struct_name}>;
            unsafe {
            out.push_str("(#{@name}");
        #{parts.join("\n")}
            out.push(')');
            }
        }
        RS
      end

      def build_replace
        stmts = @operands.map do |op|
          if op.child? || op.node? then "    if u.#{op.name} == old { u.#{op.name} = new; }"
          elsif op.nodes? then "    { let l = u.#{op.name}; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { if *p.add(i) == old { *p.add(i) = new; } } } }"
          end
        end.compact
        <<~RS
        #[allow(non_snake_case, unused_variables)]
        fn REPLACE_#{@name}(n: NodePtr, old: NodePtr, new: NodePtr) {
            let u = unsafe { &mut (*(n as *mut NodeOf<#{struct_name}>)).u };
        #{stmts.join("\n")}
        }
        RS
      end

      # SPECIALIZE_<name>(sc, n) -> String: emits (once per hash) a zero-sized type
      # `T_<hash>` implementing Disp whose call() is EVAL_<name> instantiated with the
      # children's SD types and the scalar operands baked in; returns the type name.
      def build_specialize
        if no_inline?
          return <<~RS
          #[allow(non_snake_case)]
          fn SPECIALIZE_#{@name}(_sc: &mut SpecCtx, _n: NodePtr) -> String { "Dyn".to_string() }
          RS
        end
        child_types = @operands.map do |op|
          if op.child? || op.node?
            "    let t_#{op.name} = specialize_node(sc, u.#{op.name});"
          elsif op.nodes?
            "    let t_#{op.name} = specialize_list(sc, &u.#{op.name});"
          end
        end.compact
        generic_names = @operands.select(&:generic_param).map { "t_#{it.name}" }
        child_glue = @operands.select(&:child?).map do |op|
          disp = "<{} as Disp>::call(#{(prefix_names.take(1) + ["(*np).u.#{op.name}"] + extra_call_args).join(', ')})"
          "    body.push_str(&format!(\"        let v_#{op.name} = #{unwrap_expr(disp).gsub('"', '\\"')};\\n\", t_#{op.name}));"
        end
        tf = generic_names.empty? ? '""'.dup : "format!(\"::<#{generic_names.map { '{}' }.join(', ')}>\", #{generic_names.join(', ')})"
        params = @prefix_params.join(', ')
        <<~RS
        #[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
        fn SPECIALIZE_#{@name}(sc: &mut SpecCtx, n: NodePtr) -> String {
            let name = format!("T_{:016x}", hash_node(n));
            if sc.emitted.contains(&name) { return name; }
            let u = unsafe { &(*(n as *mut NodeOf<#{struct_name}>)).u };
        #{child_types.join("\n")}
            let np = n as *mut NodeOf<#{struct_name}>;
            let mut a: Vec<String> = vec![#{prefix_names.map { "\"#{it}\".to_string()" }.join(', ')}];
            unsafe {
        #{@operands.map { "    " + it.specialize_arg }.join("\n")}
            }
            let mut body = String::new();
            body.push_str(&format!("pub struct {};\\n", name));
            if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\\n'); }
            body.push_str(&format!("impl Disp for {} {{\\n    #[inline#{always_inline? ? '(always)' : ''}]\\n    fn call(#{params}) -> #{result_type} {{ unsafe {{\\n", name));
            body.push_str(&format!("        let np = {} as *mut NodeOf<#{struct_name}>;\\n", "#{prefix_names[1]}"));
        #{child_glue.join("\n")}
            body.push_str(&format!("        EVAL_#{@name}{}({})\\n    }} }}\\n}}\\n", #{tf}, a.join(", ")));
            sc.emitted.insert(name.clone());
            sc.out.push_str(&body);
            name
        }
        RS
      end
    end

    def parse_def_head(lines, option)
      head = lines.shift.chomp
      if /\A(\w+)\((.*)\)\s*(?:->\s*(.+?))?\s*\z/ =~ head
        self.class::Node.new($1, $2, $3, option)
      else
        raise "illformed node header: #{head}"
      end
    end

    def parse
      lines = File.readlines(@opt[:input])
      @header = []
      while line = lines.shift
        if /\ANODE_DEF(\s+(@.+))?\s*\z/ =~ line.chomp
          node = parse_def_head(lines, $2)
          raise "duplicate node #{node.name}" if @nodes[node.name]
          @nodes[node.name] = node
          node.parse_body(lines)
        else
          @header << line.chomp
        end
      end
    end

    def banner = "// This file is auto-generated from #{File.basename(@file)} by astrustgen. Do not edit.\n"

    def first = @nodes.values.first

    # Embedder hooks: extra NodeHead fields (Rust field decls) and their initializers.
    def head_extra_fields = ""
    def head_extra_inits = ""

    def build_head
      n = first
      params = n.prefix_params
      ptypes = params.map { it.split(':', 2).last.strip }
      pnames = n.prefix_names
      <<~RS
      #{banner}
      pub type DispatchFn = fn(#{ptypes.join(', ')}) -> #{n.result_type};
      pub type NodePtr = *mut Node;
      pub type NodeList = &'static [NodePtr];

      pub struct NodeKind {
          pub name: &'static str,
          pub default_dispatcher: DispatchFn,
          pub hash_func: fn(NodePtr) -> u64,
          pub specializer: fn(&mut SpecCtx, NodePtr) -> String,
          pub dumper: fn(&mut String, NodePtr),
          pub replacer: fn(NodePtr, NodePtr, NodePtr),
          pub no_inline: bool,
      }

      pub const NF_HAS_HASH: u32 = 1;
      pub const NF_SPECIALIZING: u32 = 2;
      pub const NF_NO_INLINE: u32 = 4;

      #[repr(C)]
      pub struct NodeHead {
          pub dispatcher: DispatchFn,
          pub kind: &'static NodeKind,
          pub hash: u64,
          pub flags: u32,
      #{head_extra_fields}
      }
      #[repr(C)]
      pub struct Node { pub head: NodeHead }
      #[repr(C)]
      pub struct NodeOf<T> { pub head: NodeHead, pub u: T }

      pub fn node_allocate<T>(kind: &'static NodeKind, u: T) -> NodePtr {
          let n = Box::into_raw(Box::new(NodeOf { head: NodeHead {
              dispatcher: kind.default_dispatcher, kind, hash: 0,
              flags: if kind.no_inline { NF_NO_INLINE } else { 0 },
              #{head_extra_inits}
          }, u })) as NodePtr;
          node_allocated(n);
          n
      }

      /// Structural (Merkle) hash, cached in the head.
      pub fn hash_node(n: NodePtr) -> u64 {
          if n.is_null() { return hash_str("null"); }
          unsafe {
              if (*n).head.flags & NF_HAS_HASH != 0 { return (*n).head.hash; }
              let h = ((*n).head.kind.hash_func)(n);
              (*n).head.hash = h;
              (*n).head.flags |= NF_HAS_HASH;
              h
          }
      }
      pub fn hash_node_list(ns: NodeList) -> u64 {
          let mut h = hash_u64(ns.len() as u64);
          for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
          h
      }
      pub fn dump_node(out: &mut String, n: NodePtr) {
          if n.is_null() { out.push_str("null"); return; }
          unsafe { ((*n).head.kind.dumper)(out, n) }
      }
      pub fn node_replace_child(parent: NodePtr, old: NodePtr, new: NodePtr) {
          unsafe { ((*parent).head.kind.replacer)(parent, old, new) }
      }
      /// SD type name for the subtree rooted at n ("Dyn" when it must stay dynamic).
      pub fn specialize_node(sc: &mut SpecCtx, n: NodePtr) -> String {
          unsafe {
              if n.is_null() || (*n).head.flags & (NF_NO_INLINE | NF_SPECIALIZING) != 0 { return "Dyn".to_string(); }
              (*n).head.flags |= NF_SPECIALIZING;
              let t = ((*n).head.kind.specializer)(sc, n);
              (*n).head.flags &= !NF_SPECIALIZING;
              t
          }
      }
      /// Specialize n as a public entry: an exported `SD_<hash>` with the dispatcher's signature.
      pub fn specialize_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
          let fname = format!("SD_{:016x}", hash_node(n));
          if sc.emitted.contains(&fname) { return fname; }
          let t = specialize_node(sc, n);
          sc.out.push_str(&format!("#[no_mangle]\npub fn {}(#{params.join(', ')}) -> #{n.result_type} {{ <{} as Disp>::call(#{pnames.join(', ')}) }}\n", fname, t));
          sc.emitted.insert(fname.clone());
          fname
      }

      /// A dispatcher as a type: `Dyn` calls through the node head, an SD type statically.
      pub trait Disp { fn call(#{params.join(', ')}) -> #{n.result_type}; }
      /// A list of children: `DynList` loops over the heads, an SD list type is unrolled.
      pub trait DispList {
          fn len(ns: NodeList) -> usize;
          fn nth(#{params[0]}, ns: NodeList, i: usize#{params.drop(2).map { ", #{it}" }.join}) -> #{n.result_type};
      }
      pub struct Dyn;
      impl Disp for Dyn {
          #[inline(always)]
          fn call(#{params.join(', ')}) -> #{n.result_type} { unsafe { ((*#{pnames[1]}).head.dispatcher)(#{pnames.join(', ')}) } }
      }
      pub struct DynList;
      impl DispList for DynList {
          #[inline(always)] fn len(ns: NodeList) -> usize { ns.len() }
          #[inline(always)]
          fn nth(#{params[0]}, ns: NodeList, i: usize#{params.drop(2).map { ", #{it}" }.join}) -> #{n.result_type} {
              let x = ns[i];
              unsafe { ((*x).head.dispatcher)(#{([pnames[0], 'x'] + pnames.drop(2)).join(', ')}) }
          }
      }

      #{@nodes.values.map(&:build_struct).join("\n")}
      #{@nodes.values.map(&:build_alloc).join("\n")}
      #{@nodes.values.map(&:build_kind).join("\n")}
      RS
    end

    def build_eval = banner + @header.join("\n") + "\n\n" + @nodes.values.map(&:build_eval).join("\n")
    def build_dispatch = banner + @nodes.values.map(&:build_dispatch).join("\n")
    def build_hash = banner + @nodes.values.map(&:build_hash).join("\n")
    def build_dump = banner + @nodes.values.map(&:build_dump).join("\n")
    def build_replace = banner + @nodes.values.map(&:build_replace).join("\n")

    # The specializer also emits, for an SD list type, an unrolled `nth`.
    def build_specialize
      banner + @nodes.values.map(&:build_specialize).join("\n") + <<~RS

      /// SD type for a NODES operand: a `DispList` whose nth() dispatches statically.
      pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList) -> String {
          let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
          let mut h = hash_str("list");
          for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
          let name = format!("L_{:016x}", h);
          if sc.emitted.contains(&name) { return name; }
          let mut s = format!("pub struct {};\\nimpl DispList for {} {{\\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\\n", name, name, ts.len());
          s.push_str(&format!("    #[inline(always)]\\n    fn nth(#{first.prefix_params[0]}, ns: NodeList, i: usize#{first.prefix_params.drop(2).map { ", #{it}" }.join}) -> #{first.result_type} {{\\n        match i {{\\n"));
          for (i, t) in ts.iter().enumerate() {
              s.push_str(&format!("            {} => <{} as Disp>::call(#{([first.prefix_names[0], 'ns[{}]'] + first.prefix_names.drop(2)).join(', ')}),\\n", i, t, i));
          }
          s.push_str("            _ => unsafe { std::hint::unreachable_unchecked() },\\n        }\\n    }\\n}\\n");
          sc.emitted.insert(name.clone());
          sc.out.push_str(&s);
          name
      }
      RS
    end

    def gen
      dir = @opt[:output_dir]
      self.class.gen_tasks.each do |task|
        File.write(File.join(dir, "#{@opt[:output_prefix]}_#{task.name}.rs"), send("build_#{task.name}"))
      end
      File.write(File.join(dir, "#{@opt[:output_prefix]}_head.rs"), build_head)
    end
  end

  def self.parse_opt argv
    opt = { input: 'node.def', output_prefix: 'node', output_dir: Dir.pwd }
    op = OptionParser.new
    op.on('--input=FILE') { opt[:input] = it }
    op.on('--output-dir=DIR') { opt[:output_dir] = it }
    op.on('--output-prefix=NAME') { opt[:output_prefix] = it }
    op.parse!(argv)
    opt
  end

  def self.start argv, node_def_class: NodeDef
    opt = parse_opt(argv)
    nd = node_def_class.new(opt[:input], opt)
    nd.parse
    nd.gen
  end
end
