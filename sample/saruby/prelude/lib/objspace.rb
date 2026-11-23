# frozen_string_literal: true
# objspace (ext/objspace/lib/objspace.rb of CRuby 4.0.2): the Ruby half over objspace.so
# (builtins/objspace.rs).

require 'objspace.so'

module ObjectSpace
  class << self
    private :_dump
    private :_dump_all
    private :_dump_shapes
  end

  module_function

  def dump(obj, output: :string)
    out = case output
    when :file, nil
      require 'tempfile'
      Tempfile.create(%w(rubyobj .json))
    when :stdout
      STDOUT
    when :string
      +''
    when IO
      output
    else
      raise ArgumentError, "wrong output option: #{output.inspect}"
    end

    ret = _dump(obj, out)
    return nil if output == :stdout
    ret
  end

  def dump_all(output: :file, full: false, since: nil, shapes: true)
    out = case output
    when :file, nil
      require 'tempfile'
      Tempfile.create(%w(rubyheap .json))
    when :stdout
      STDOUT
    when :string
      +''
    when IO
      output
    else
      raise ArgumentError, "wrong output option: #{output.inspect}"
    end

    shapes = 0 if shapes == true
    ret = _dump_all(out, full, since, shapes)
    return nil if output == :stdout
    ret
  end

  def dump_shapes(output: :file, since: 0)
    out = case output
    when :file, nil
      require 'tempfile'
      Tempfile.create(%w(rubyshapes .json))
    when :stdout
      STDOUT
    when :string
      +''
    when IO
      output
    else
      raise ArgumentError, "wrong output option: #{output.inspect}"
    end

    ret = _dump_shapes(out, since)
    return nil if output == :stdout
    ret
  end
end
