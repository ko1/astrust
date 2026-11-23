# saruby: what base.rb reads from RubyVM::AbstractSyntaxTree and the prism gem, defined under ErrorHighlight so
# its unqualified `RubyVM` / `Prism` find these and nothing changes outside it.  A node is the one with the
# backtrace location's prism node id in a re-parse of the file (builtins/error_highlight.rs), with the
# prism gem's field names and Location arithmetic (prism/parse_result.rb).
require "saruby/error_highlight"

module ErrorHighlight
  # the boot stub of CoreExt#detailed_message (builtins/error_highlight.rs), once this file is loaded: the real one
  def self.__detailed_message(exc, opts)
    CoreExt.instance_method(:detailed_message).bind_call(exc, **opts)
  end

  module RubyVM
    module AbstractSyntaxTree
      class Node; end

      # saruby compiles with prism, as CRuby 4.0 does: base.rb then takes the prism path
      def self.of(*, **)
        raise RuntimeError, "cannot get AST for ISEQ compiled by prism"
      end

      def self.node_id_for_backtrace_location(location)
        ErrorHighlight.__node_id(location)
      end
    end
  end

  module Prism
    VERSION = "1.9.0"

    class Source
      attr_reader :source, :start_line, :offsets

      def initialize(source, offsets)
        @source = source
        @start_line = 1
        @offsets = offsets
      end

      def lines = source.lines
      def slice(byte_offset, length) = (source.byteslice(byte_offset, length) or raise)
      def line(byte_offset) = start_line + find_line(byte_offset)
      def line_start(byte_offset) = offsets[find_line(byte_offset)]
      def line_end(byte_offset) = offsets[find_line(byte_offset) + 1] || source.bytesize
      def column(byte_offset) = byte_offset - line_start(byte_offset)

      private

      def find_line(byte_offset)
        index = offsets.bsearch_index { |offset| offset > byte_offset } || offsets.length
        index - 1
      end
    end

    class Location
      attr_reader :source
      protected :source
      attr_reader :start_offset, :length

      def initialize(source, start_offset, length)
        @source = source
        @start_offset = start_offset
        @length = length
      end

      def copy(source: self.source, start_offset: self.start_offset, length: self.length)
        Location.new(source, start_offset, length)
      end

      def chop = copy(length: length == 0 ? length : length - 1)
      def source_lines = source.lines
      def slice = source.slice(start_offset, length)
      def end_offset = start_offset + length
      def start_line = source.line(start_offset)
      def end_line = source.line(end_offset)
      def start_column = source.column(start_offset)
      def end_column = source.column(end_offset)

      def join(other)
        raise "Incompatible sources" if source != other.source
        raise "Incompatible locations" if start_offset > other.start_offset

        Location.new(source, start_offset, other.end_offset - start_offset)
      end

      def adjoin(string)
        line_suffix = source.slice(end_offset, source.line_end(end_offset) - end_offset)

        line_suffix_index = line_suffix.byteindex(string)
        raise "Could not find #{string}" if line_suffix_index.nil?

        Location.new(source, start_offset, length + line_suffix_index + string.bytesize)
      end
    end

    class Node
      attr_reader :type, :node_id, :location

      def initialize(type, node_id, location)
        @type = type
        @node_id = node_id
        @location = location
      end

      def source_lines = location.source_lines
      alias script_lines source_lines
    end

    # node class per type (CallNode for :call_node): readers for its fields, and for each `x_loc` the slice `x`
    def self.node_class(type, fields)
      name = type.to_s.split("_").map(&:capitalize).join
      return const_get(name, false) if const_defined?(name, false)
      k = Class.new(Node)
      fields.each_key do |f|
        k.attr_reader f
        s = f.to_s.delete_suffix("_loc")
        k.define_method(s) { instance_variable_get(:"@#{f}")&.slice } if s != f.to_s && !fields.key?(s.to_sym)
      end
      const_set(name, k)
    end

    # raw: a node [type, id, start, length, fields], a location [start, length], a list, or a plain value
    def self.build(source, raw)
      return raw unless raw.is_a?(Array)
      if raw.size == 5 && raw[0].is_a?(Symbol) && raw[4].is_a?(Hash)
        type, id, start, len, fields = raw
        node = node_class(type, fields).new(type, id, Location.new(source, start, len))
        fields.each { |f, v| node.instance_variable_set(:"@#{f}", build(source, v)) }
        node
      elsif raw.size == 2 && raw[0].is_a?(Integer)
        Location.new(source, raw[0], raw[1])
      else
        raw.map { build(source, it) }
      end
    end

    # The node with prism node id `node_id` in a parse of the file `path` (nil: none).
    def self.find(path, node_id)
      src, offsets, raw = ErrorHighlight.__find(path, node_id)
      raw && build(Source.new(src, offsets), raw)
    end
  end
end
