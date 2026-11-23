# objspace/trace (ext/objspace/lib/objspace/trace.rb of CRuby 4.0.2): `p` shows where an object
# was allocated.  Starts the allocation tracer.

require 'objspace.so'

module Kernel
  remove_method :p
  define_method(:p) do |*objs|
    objs.each do |obj|
      file = ObjectSpace.allocation_sourcefile(obj)
      line = ObjectSpace.allocation_sourceline(obj)
      if file
        puts "#{ obj.inspect } @ #{ file }:#{ line }"
      else
        puts obj.inspect
      end
    end
  end
end

ObjectSpace.trace_object_allocations_start

warn "objspace/trace is enabled"
