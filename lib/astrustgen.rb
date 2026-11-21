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
#   x@spec: NODE      speculated callee: not stored in the node.  At specialize time the embedder's
#                     `spec_target_expr` names a node (or null; never an ancestor) to compile in; the body compares
#                     SPEC_HASH!(x) (0 in the interpreter) with the runtime target's hash and then
#                     calls it statically with EVAL_SPEC!(c, x, <node ptr>, <common args...>).
#   x@bake: u32       a scalar that is not stored in the node: 0 in the interpreter, and in specialized
#                     code the value the embedder's `bake_value_expr` computes at specialize time
#                     (e.g. what kind of callee a call site has seen), so the body can keep one case inline.
#   x@specin: T       a value a speculating caller hands to this node directly: EVAL_SPEC!(c, x, <node ptr>,
#                     <common args...>, <value>) passes it through `Disp::spec`, with no memory in between.
#                     Reached any other way (interpreter, `call`) the body gets the embedder's
#                     `spec_in_load_expr` (e.g. a read from the context); a `spec` that cannot pass it on (Dyn,
#                     a node without @specin) stores it with `spec_in_store_stmt` first.  One type per node.def.
#   NODE_TEST { ... }  (optional, right after a node's body) the node evaluated as a condition:
#                     returns the embedder's `test_type` directly; children with EVAL_TEST!(c, x).
#                     Nodes without one fall back to `test_default` over their value.
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
#
# Contiguous trees (embedder hook `contiguous?`): `layout_tree(root)` copies a finished tree into one block,
# parents before children.  A descendant then sits at an offset from its ancestor that depends only on the
# tree's shape (which the hash covers), so specialized code addresses every node, and so every inline cache,
# as `root + constant` instead of loading child pointers level by level.  Only laid-out trees are specialized.
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
          @spec = name.end_with?('@spec')
          @bake = name.end_with?('@bake')
          @specin = name.end_with?('@specin')
          @name = name.sub(/@(ref|child|specin|spec|bake)$/, '')
          @type = type.strip
        end

        def ref? = @ref
        def child? = @child
        def spec? = @spec
        def bake? = @bake
        def specin? = @specin
        def virtual? = @spec || @bake || @specin   # not a field of the node
        def node? = !@ref && !@child && !@spec && @type == 'NODE'
        def nodes? = !@ref && @type == 'NODES'
        def any_node? = child? || node? || nodes?
        def scalar? = !ref? && !bake? && SCALARS.key?(@type)
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
          elsif spec? then "#{@name}_hash: u64"
          elsif child? then "#{@name}: #{@type}"
          elsif node? then "#{@name}: NodePtr"
          elsif nodes? then "#{@name}: NodeList"
          else "#{@name}: #{@type}"
          end
        end

        def generic_param
          if node? || spec? then "D_#{@name}: Disp"
          elsif nodes? then "D_#{@name}: DispList"
          end
        end

        def field(u = 'u') = "#{u}.#{@name}"

        # Rust expression (u64) hashing this operand; `u` names the operand struct.
        def hash_expr(u = 'u')
          f = field(u)
          if child? || node? then "hash_node(#{f})"
          elsif nodes? then "hash_node_list(#{f})"
          elsif ref? || virtual? then nil
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
          elsif spec? then "a.push(format!(\"{}u64\", h_#{@name}));"
          elsif bake? then "a.push(format!(\"#{SCALARS[@type]}\", #{@owner.bake_value_expr(self)}));"
          elsif specin? then "a.push(\"#{ASTrustGen.rust_str(@owner.spec_in_load_expr(self))}\".to_string());"
          elsif child? then "a.push(\"#{ASTrustGen.rust_str(@owner.child_storage_expr(self))}\".to_string());"
          elsif node? then @owner.contiguous? ? "a.push(child_ptr_expr(n, u.#{@name}));" : "a.push(\"(*np).u.#{@name}\".to_string());"
          elsif nodes? then @owner.contiguous? ? "a.push(\"list_base(#{@owner.prefix_names[1]})\".to_string());" : "a.push(\"(*np).u.#{@name}\".to_string());"
          elsif scalar? then "a.push(format!(\"#{SCALARS[@type]}\", #{f}));"
          elsif f64? then "a.push(format!(\"f64::from_bits({}u64)\", #{f}.to_bits()));"
          else @owner.opaque_specialize_arg(self, f)
          end
        end

        def dump_stmt
          f = "(*np).u.#{@name}"
          if virtual? then ""
          elsif child? || node? then "dump_node(out, #{f});"
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
      # Condition mode (NODE_TEST / EVAL_TEST!): its result type, and how a value result becomes one. nil = off.
      def test_type = nil
      def test_default(e) = e
      # Opaque (non-scalar, non-node) operands: hash / specialize / dump.
      def opaque_hash_expr(op, f) = "0u64"
      def opaque_specialize_arg(op, f) = "a.push(\"(*np).u.#{op.name}\".to_string());"
      def opaque_dump_stmt(op, f) = "out.push_str(\"<#{op.name}>\");"
      # @spec operand: Rust expr (NodePtr, null = none) evaluated at specialize time with `np` in scope.
      def spec_target_expr(op) = "std::ptr::null_mut()"
      # @spec operand whose target is being specialized now (recursion): the target regardless of size, or nil
      # (the call stays dynamic).  Such a site gets `R_<hash>`, a direct call of the target's specialized entry
      # (`S_<hash>::run`, with the @specin value as a parameter); `rec_call_stmt` runs before that call.
      def spec_rec_target_expr(op) = nil
      def rec_call_stmt = ""
      # @bake operand: Rust expr of the operand's type, evaluated at specialize time with `np` in scope.
      def bake_value_expr(op) = "0"
      # @specin operand: the value when no speculating caller passed it, and how a `spec` that cannot
      # pass it on leaves it where that expression reads it (`var` names the value).
      def spec_in_load_expr(op) = "Default::default()"
      def spec_in_store_stmt(var) = "let _ = #{var};"
      # Trees are laid out by layout_tree and specialized code reaches children at constant offsets.
      def contiguous? = false
      def dispatch_prologue = ""
      # @child storage: where an eager child's value lives between its evaluation and the
      # node body (the counterpart of ASTroGen's child_storage_decl / child_storage_expr).
      #   child_storage_stmt(op, value)  the statement that evaluates and keeps it; emitted once
      #                                  per @child, in operand order, in DISPATCH and SD glue
      #   child_storage_expr(op)         the argument passed to EVAL
      # Default: a Rust local — right with no GC, or with one that scans the machine stack.
      # A precise-GC embedder stages the value in a root-scanned slot instead, e.g.
      #   def child_storage_stmt(op, value) = "*fp.add(#{base + child_index(op)}) = #{value};"
      #   def child_storage_expr(op) = "*fp.add(#{base + child_index(op)})"
      def child_storage_stmt(op, value) = "let v_#{op.name} = #{value};"
      def child_storage_expr(op) = "v_#{op.name}"
      # Position of `op` among this node's @child operands.
      def child_index(op) = @operands.select(&:child?).index(op)

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
          raise "ill-formed operand: #{p}" unless /\A([a-zA-Z_][a-zA-Z0-9_]*(?:@ref|@child|@specin|@spec|@bake)?)\s*:\s*(.+)\z/ =~ p
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

      attr_reader :test_body
      def parse_test_body lines
        main, @body = @body, nil
        parse_body(lines)
        @test_body, @body = @body, main
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
        }.gsub(/SPEC_HASH!\(\s*(\w+)\s*\)/) { "#{$1}_hash" }
         .gsub(/EVAL_SPEC!\(\s*(\w+)\s*,\s*(\w+)\s*,/) { "<D_#{$2} as Disp>::spec(#{$1}," }
         .gsub(/EVAL_LEN!\(\s*(\w+)\s*\)/) { "<D_#{$1} as DispList>::len(#{$1})" }
         .gsub(/EVAL_TEST!\(\s*(\w+)\s*,\s*(\w+)\s*\)/) { "<D_#{$2} as Disp>::test(#{$1}, #{$2}#{extra.empty? ? '' : ", #{extra.join(', ')}"})" }
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
        #{build_eval_test(params)}
        RS
      end

      def build_eval_test(params)
        return "" unless @test_body
        <<~RS
        #[inline(always)]
        #[allow(unused_variables, unused_mut, non_snake_case, unused_unsafe)]
        pub fn EVAL_TEST_#{@name}#{generics}(#{params}) -> #{test_type} {
        #{substitute_body(@test_body)}}
        RS
      end

      def build_struct
        fields = @operands.reject(&:virtual?).map { "    pub #{it.name}: #{it.storage_type}," }
        <<~RS
        #[allow(non_camel_case_types)]
        #[repr(C)]
        pub struct #{struct_name} {
        #{fields.join("\n")}
        }
        RS
      end

      def alloc_params = @operands.reject { it.ref? || it.virtual? }.map { "#{it.name}: #{it.storage_type}" }.join(', ')

      def build_alloc
        inits = @operands.reject(&:virtual?).map { it.ref? ? "#{it.name}: Default::default()" : it.name }
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
            size: std::mem::size_of::<NodeOf<#{struct_name}>>(),
            children: CHILDREN_#{@name},
        };
        const _: () = assert!(std::mem::size_of::<NodeOf<#{struct_name}>>() % 8 == 0 && std::mem::align_of::<NodeOf<#{struct_name}>>() <= 8);
        RS
      end

      # CHILDREN_<name>(n, f): f on every child pointer slot, in operand order (lists element by element).
      def build_children
        stmts = @operands.map do |op|
          if op.child? || op.node? then "    f(&mut u.#{op.name});"
          elsif op.nodes? then "    { let l = u.#{op.name}; let p = l.as_ptr() as *mut NodePtr; for i in 0..l.len() { unsafe { f(&mut *p.add(i)); } } }"
          end
        end.compact
        <<~RS
        #[allow(non_snake_case, unused_variables)]
        fn CHILDREN_#{@name}(n: NodePtr, f: &mut dyn FnMut(&mut NodePtr)) {
            let u = unsafe { &mut (*(n as *mut NodeOf<#{struct_name}>)).u };
        #{stmts.join("\n")}
        }
        RS
      end

      # Argument list for EVAL in DISPATCH and SD bodies.
      def eval_call_args
        args = prefix_names.dup
        @operands.each do |op|
          if op.ref? then args << "&mut (*np).u.#{op.name}"
          elsif op.spec? then args << "0u64"
          elsif op.bake? then args << "0"
          elsif op.specin? then args << spec_in_load_expr(op)
          elsif op.child? then args << child_storage_expr(op)
          else args << "(*np).u.#{op.name}"
          end
        end
        args.join(', ')
      end

      def turbofish(types) = types.empty? ? "" : "::<#{types.join(', ')}>"

      def child_eval_stmts(disp_of)
        @operands.select(&:child?).map do |op|
          "    #{child_storage_stmt(op, unwrap_expr("<#{disp_of.(op)} as Disp>::call(#{(prefix_names.take(1) + ["(*np).u.#{op.name}"] + extra_call_args).join(', ')})"))}"
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
      def build_specialize_test(params, child_glue, tf)
        return "" unless @test_body
        <<~RS.chomp
            body.push_str(&format!("    #[inline(always)]\\n    fn test(#{params}) -> #{test_type} {{ unsafe {{\\n"));
            body.push_str(&format!("        let np = {} as *mut NodeOf<#{struct_name}>;\\n", "#{prefix_names[1]}"));
        #{child_glue.join("\n")}
            body.push_str(&format!("        EVAL_TEST_#{@name}{}({})\\n    }} }}\\n", #{tf}, a.join(", ")));
        RS
      end

      # A node with a @specin operand: its `spec` takes the value as a parameter instead of loading it.
      def build_specialize_spec(params, child_glue, tf)
        op = @operands.find(&:specin?) or return ""
        idx = @prefix_params.size + @operands.index(op)
        <<~RS.chomp
            let a_spec: Vec<String> = a.iter().enumerate().map(|(i, x)| if i == #{idx} { "#{op.name}".to_string() } else { x.clone() }).collect();
            body.push_str(&format!("    #[inline(always)]\\n    fn spec(#{params}, #{op.name}: #{op.type}) -> #{result_type} {{ unsafe {{\\n"));
            body.push_str(&format!("        let np = {} as *mut NodeOf<#{struct_name}>;\\n", "#{prefix_names[1]}"));
        #{child_glue.join("\n")}
            body.push_str(&format!("        EVAL_#{@name}{}({})\\n    }} }}\\n", #{tf}, a_spec.join(", ")));
        RS
      end

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
            "    let t_#{op.name} = specialize_list(sc, &u.#{op.name}#{contiguous? ? ", n" : ""});"
          elsif op.spec?
            rec = spec_rec_target_expr(op)
            dyn = rec ? "{ let q: NodePtr = unsafe { #{rec} }; if sc.spec_max_depth > 0 && !q.is_null() && sc.open.contains(&hash_node(q)) { (specialize_rec(sc, q), hash_node(q)) } else { (\"Dyn\".to_string(), 0u64) } }" : "(\"Dyn\".to_string(), 0u64)"
            "    let (t_#{op.name}, h_#{op.name}) = { let np = n as *mut NodeOf<#{struct_name}>; let p: NodePtr = unsafe { #{spec_target_expr(op)} }; if p.is_null() || sc.spec_depth >= sc.spec_max_depth || sc.open.contains(&hash_node(p)) { #{dyn} } else { sc.spec_depth += 1; let t = specialize_node(sc, p); sc.spec_depth -= 1; (t, hash_node(p)) } };"
          end
        end.compact
        generic_names = @operands.select(&:generic_param).map { "t_#{it.name}" }
        child_glue = @operands.select(&:child?).map do |op|
          # \u0001 stands for the child's dispatcher type until the statement is escaped for format!
          # \u0002 stands for the child's node pointer (contiguous trees: an offset from this node)
          disp = "<\u0001 as Disp>::call(#{(prefix_names.take(1) + [contiguous? ? "\u0002" : "(*np).u.#{op.name}"] + extra_call_args).join(', ')})"
          stmt = ASTrustGen.rust_str(child_storage_stmt(op, unwrap_expr(disp))).gsub('{', '{{').gsub('}', '}}').sub("\u0001", '{}').sub("\u0002", '{}')
          "    body.push_str(&format!(\"        #{stmt}\\n\", t_#{op.name}#{contiguous? ? ", child_ptr_expr(n, u.#{op.name})" : ""}));"
        end
        key_types = @operands.select { it.child? || it.node? || it.nodes? || it.spec? }.map { "t_#{it.name}.as_str()" }.join(', ')
        tf = generic_names.empty? ? '""'.dup : "format!(\"::<#{generic_names.map { '{}' }.join(', ')}>\", #{generic_names.join(', ')})"
        params = @prefix_params.join(', ')
        <<~RS
        #[allow(non_snake_case, unused_mut, unused_unsafe, unused_variables)]
        fn SPECIALIZE_#{@name}(sc: &mut SpecCtx, n: NodePtr) -> String {
            let own_hash = hash_node(n);
            let was_open = !sc.open.insert(own_hash);
            let u = unsafe { &(*(n as *mut NodeOf<#{struct_name}>)).u };
        #{child_types.join("\n")}
            let np = n as *mut NodeOf<#{struct_name}>;
            let mut a: Vec<String> = vec![#{prefix_names.map { "\"#{it}\".to_string()" }.join(', ')}];
            unsafe {
        #{@operands.map { "    " + it.specialize_arg }.join("\n")}
            }
            if !was_open { sc.open.remove(&own_hash); }
            // named by what is baked too (children's types, @bake values), not by the hash alone:
            // two sites with one hash may have seen different things
            let name = format!("T_{:016x}", hash_merge(own_hash, hash_str(&format!("{}|{}", <[&str]>::join(&[#{key_types}], ","), a.join(",")))));
            if sc.emitted.contains(&name) { return name; }
            sc.emitted.insert(name.clone());
            let mut body = String::new();
            body.push_str(&format!("pub struct {};\\n", name));
            if sc.comments { body.push_str("// "); dump_node(&mut body, n); body.push('\\n'); }
            body.push_str(&format!("impl Disp for {} {{\\n    #[inline(always)]\\n    fn call(#{params}) -> #{result_type} {{ unsafe {{\\n", name));
            body.push_str(&format!("        let np = {} as *mut NodeOf<#{struct_name}>;\\n", "#{prefix_names[1]}"));
        #{child_glue.join("\n")}
            body.push_str(&format!("        EVAL_#{@name}{}({})\\n    }} }}\\n", #{tf}, a.join(", ")));
        #{build_specialize_test(params, child_glue, tf)}
        #{build_specialize_spec(params, child_glue, tf)}
            body.push_str("}\n");
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
        elsif line.chomp == "NODE_TEST"
          node = @nodes.values.last or raise "NODE_TEST without a node"
          raise "NODE_TEST needs the embedder's test_type (#{node.name})" unless node.test_type
          node.parse_test_body(lines)
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
      sins = @nodes.values.flat_map(&:operands).select(&:specin?).map(&:type).uniq
      raise "@specin operands of different types: #{sins.join(', ')}" if sins.size > 1
      spec_in = sins.first
      sin_op = @nodes.values.lazy.map { |x| [x, x.operands.find(&:specin?)] }.find { it[1] }
      esc = ->(s) { ASTrustGen.rust_str(s).gsub('{', '{{').gsub('}', '}}') }
      rec_load = sin_op ? ", #{esc.(sin_op[0].spec_in_load_expr(sin_op[1]))}" : ''
      rec_stmt = n.rec_call_stmt.empty? ? '' : esc.(n.rec_call_stmt) + ' '
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
          pub size: usize,
          pub children: fn(NodePtr, &mut dyn FnMut(&mut NodePtr)),
      }

      pub const NF_HAS_HASH: u32 = 1;
      pub const NF_SPECIALIZING: u32 = 2;
      pub const NF_NO_INLINE: u32 = 4;
      pub const NF_LAID: u32 = 8;        // part of a block made by layout_tree

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

      /// Copy the tree under `root` into one block, parents before children in operand order, and
      /// return the copy.  The original stays valid (other holders of its nodes are unaffected).
      pub fn layout_tree(root: NodePtr) -> NodePtr {
          fn size(n: NodePtr) -> usize {
              if n.is_null() { return 0; }
              let k = unsafe { (*n).head.kind };
              let mut s = k.size;
              (k.children)(n, &mut |c| s += size(*c));
              s
          }
          fn copy(n: NodePtr, at: &mut *mut u8) -> NodePtr {
              unsafe {
                  let k = (*n).head.kind;
                  let d = *at;
                  std::ptr::copy_nonoverlapping(n as *const u8, d, k.size);
                  *at = d.add(k.size);
                  let d = d as NodePtr;
                  (*d).head.flags |= NF_LAID;
                  (k.children)(d, &mut |c| if !c.is_null() { *c = copy(*c, at); });
                  d
              }
          }
          if root.is_null() { return root; }
          let mut at = Box::leak(vec![0u64; size(root) / 8].into_boxed_slice()).as_mut_ptr() as *mut u8;
          copy(root, &mut at)
      }
      pub fn node_laid(n: NodePtr) -> bool { !n.is_null() && unsafe { (*n).head.flags & NF_LAID != 0 } }
      /// Specialized code's pointer to child `c` of `n` (contiguous trees).
      pub fn child_ptr_expr(n: NodePtr, c: NodePtr) -> String {
          if c.is_null() { "std::ptr::null_mut()".to_string() } else { format!("#{pnames[1]}.byte_offset({}isize)", c as isize - n as isize) }
      }
      /// A list operand in specialized code over a contiguous tree: no elements, the parent's address.
      #[inline(always)]
      pub fn list_base(n: NodePtr) -> NodeList { unsafe { std::slice::from_raw_parts(n as *const NodePtr, 0) } }

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
              if n.is_null() || (*n).head.flags & (NF_NO_INLINE | NF_SPECIALIZING) != 0#{n.contiguous? ? ' || (*n).head.flags & NF_LAID == 0' : ''} { return "Dyn".to_string(); }
              (*n).head.flags |= NF_SPECIALIZING;
              let t = ((*n).head.kind.specializer)(sc, n);
              (*n).head.flags &= !NF_SPECIALIZING;
              t
          }
      }
      /// Specialize n as a public entry: an exported `SD_<hash>` with the dispatcher's signature.
      pub fn specialize_entry(sc: &mut SpecCtx, n: NodePtr) -> String {
          let fname = format!("SD_{:016x}", hash_node(n));
          if sc.emitted.contains(&fname)#{n.contiguous? ? ' || !node_laid(n)' : ''} { return fname; }
          let t = specialize_node(sc, n);
          let s = format!("S_{:016x}", hash_node(n));
          if sc.emitted.contains(&format!("R_{:016x}", hash_node(n))) && !sc.emitted.contains(&s) {
              // n calls itself: its one body is S_<hash>::run (R_ calls it directly); SD_ passes the spec-in value on
              specialize_run(sc, &s, &t);
              sc.out.push_str(&format!("#[no_mangle]\npub fn {}(#{params.join(', ')}) -> #{n.result_type} {{ {}::run(#{pnames.join(', ')}#{rec_load}) }}\n", fname, s));
          } else {
              sc.out.push_str(&format!("#[no_mangle]\npub fn {}(#{params.join(', ')}) -> #{n.result_type} {{ <{} as Disp>::call(#{pnames.join(', ')}) }}\n", fname, t));
          }
          sc.emitted.insert(fname.clone());
          fname
      }
      /// A call from inside the specialization of q to q (recursion): `R_<hash>`, a direct call of
      /// `S_<hash>::run`, q's entry taking the spec-in value as a parameter (specialize_entry / _finish emit it).
      pub fn specialize_rec(sc: &mut SpecCtx, q: NodePtr) -> String {
          let h = hash_node(q);
          let name = format!("R_{:016x}", h);
          if sc.emitted.insert(name.clone()) {
              sc.rec.push(q as usize);
              sc.out.push_str(&format!("pub struct {0};\\nimpl Disp for {0} {{\\n    #[inline(always)]\\n    fn call(#{params.join(', ')}) -> #{n.result_type} {{ #{rec_stmt}S_{1:016x}::run(#{pnames.join(', ')}#{rec_load}) }}\\n    #[inline(always)]\\n    fn spec(#{params.join(', ')}#{spec_in ? ", a: #{spec_in}" : ''}) -> #{n.result_type} {{ #{rec_stmt}S_{1:016x}::run(#{pnames.join(', ')}#{spec_in ? ', a' : ''}) }}\\n}}\\n", name, h));
          }
          name
      }
      fn specialize_run(sc: &mut SpecCtx, s: &str, t: &str) {
          sc.out.push_str(&format!("pub struct {0};\\nimpl {0} {{\\n    #[inline(never)]\\n    pub fn run(#{params.join(', ')}#{spec_in ? ", a: #{spec_in}" : ''}) -> #{n.result_type} {{ <{1} as Disp>::spec(#{pnames.join(', ')}#{spec_in ? ', a' : ''}) }}\\n}}\\n", s, t));
          sc.emitted.insert(s.to_string());
      }
      /// After the entries: S_<hash>::run of the recursion targets whose SD_ was not the one to provide it.
      pub fn specialize_finish(sc: &mut SpecCtx) {
          while let Some(q) = sc.rec.pop() {
              let q = q as NodePtr;
              let s = format!("S_{:016x}", hash_node(q));
              if sc.emitted.contains(&s) { continue; }
              let t = specialize_node(sc, q);
              specialize_run(sc, &s, &t);
          }
      }

      /// A dispatcher as a type: `Dyn` calls through the node head, an SD type statically.
      pub trait Disp {
          fn call(#{params.join(', ')}) -> #{n.result_type};
          /// call() across a speculation boundary (the embedder only speculates small callees, to a bounded depth).
          #[inline(always)]
          fn spec(#{params.join(', ')}#{spec_in ? ", a: #{spec_in}" : ''}) -> #{n.result_type} { #{spec_in ? n.spec_in_store_stmt('a') + ' ' : ''}Self::call(#{pnames.join(', ')}) }
      #{n.test_type ? "    #[inline(always)]\n    fn test(#{params.join(', ')}) -> #{n.test_type} { #{n.test_default("Self::call(#{pnames.join(', ')})")} }" : ""}
      }
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
      #{@nodes.values.map(&:build_children).join("\n")}
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
      pub fn specialize_list(sc: &mut SpecCtx, ns: &NodeList#{first.contiguous? ? ', parent: NodePtr' : ''}) -> String {
          let ts: Vec<String> = ns.iter().map(|x| specialize_node(sc, *x)).collect();
          let mut h = hash_str("list");
          for x in ns.iter() { h = hash_merge(h, hash_node(*x)); }
          #{first.contiguous? ? 'if let Some(x) = ns.first() { h = hash_merge(h, (*x as usize).wrapping_sub(parent as usize) as u64); }' : ''}
          let name = format!("L_{:016x}", h);
          if sc.emitted.contains(&name) { return name; }
          let mut s = format!("pub struct {};\\nimpl DispList for {} {{\\n    #[inline(always)] fn len(_ns: NodeList) -> usize {{ {} }}\\n", name, name, ts.len());
          s.push_str(&format!("    #[inline(always)]\\n    fn nth(#{first.prefix_params[0]}, ns: NodeList, i: usize#{first.prefix_params.drop(2).map { ", #{it}" }.join}) -> #{first.result_type} {{\\n        match i {{\\n"));
          for (i, t) in ts.iter().enumerate() {
              #{first.contiguous? ? 'let x = child_ptr_expr(parent, ns[i]).replacen("' + first.prefix_names[1] + '.", "(ns.as_ptr() as NodePtr).", 1);' : 'let x = format!("*ns.get_unchecked({})", i);'}
              s.push_str(&format!("            {} => unsafe {{ <{} as Disp>::call(#{([first.prefix_names[0], '{}'] + first.prefix_names.drop(2)).join(', ')}) }},\\n", i, t, x));
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

  # `s` as the inside of a Rust string literal.
  def self.rust_str(s) = s.gsub('\\') { '\\\\' }.gsub('"') { '\\"' }

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
