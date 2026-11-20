# Timeout on SIGALRM: the block is interrupted at the next blocking point (sleep / IO / wait).
module Timeout
  VERSION = "0.6.0"

  class ExitException < Exception
    def exception(*) = self
  end

  class Error < RuntimeError
    def self.handle_timeout(message)
      exc = ExitException.new(message)
      begin
        yield exc
      rescue ExitException => e
        raise new(message) if exc.equal?(e)
        raise
      end
    end

    def exception(*args)
      args.empty? || args == [self] ? self : super
    end
  end

  @stack = []

  def self.__now = Process.clock_gettime(Process::CLOCK_MONOTONIC)

  def self.__arm
    if @stack.empty?
      Process.__itimer(0)
    else
      d = @stack.map(&:first).min - __now
      Process.__itimer(d <= 0 ? 0.000001 : d)
    end
  end

  def self.__fire
    now = __now
    ent = @stack.find { |e| e[0] <= now && !e[3] }
    return __arm unless ent
    ent[3] = true
    raise ent[1], ent[2]
  end

  def self.timeout(sec, klass = nil, message = nil, &block)
    return yield(sec) if sec == nil or sec.zero?
    raise ArgumentError, "Timeout sec must be a non-negative number" if sec < 0
    message ||= "execution expired"
    ent = [__now + sec, klass || Error, message, false]
    @stack << ent
    trap("ALRM") { Timeout.__fire }
    __arm
    begin
      yield(sec)
    ensure
      @stack.delete_if { |e| e.equal?(ent) }
      __arm
    end
  end

  private def timeout(*args, &block)
    Timeout.timeout(*args, &block)
  end
end
