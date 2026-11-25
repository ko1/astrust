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

  def __saruby__.timeout_now = Process.clock_gettime(Process::CLOCK_MONOTONIC)

  def __saruby__.timeout_arm(stack)
    if stack.empty?
      __itimer(0)
    else
      d = stack.map(&:first).min - timeout_now
      __itimer(d <= 0 ? 0.000001 : d)
    end
  end

  def __saruby__.timeout_fire(stack)
    now = timeout_now
    ent = stack.find { |e| e[0] <= now && !e[3] }
    return timeout_arm(stack) unless ent
    ent[3] = true
    raise ent[1], ent[2]
  end

  def self.timeout(sec, klass = nil, message = nil, &block)
    return yield(sec) if sec == nil or sec.zero?
    raise ArgumentError, "Timeout sec must be a non-negative number" if sec < 0
    message ||= "execution expired"
    # a non-blocking fiber's scheduler may run the timeout (CRuby's timeout.rb)
    if (scheduler = Fiber.current_scheduler)&.respond_to?(:timeout_after)
      perform = Proc.new { |exc| scheduler.timeout_after(sec, exc, message, &block) }
      return klass ? perform.call(klass) : Error.handle_timeout(message, &perform)
    end
    ent =[__saruby__.timeout_now + sec, klass || Error, message, false]
    @stack << ent
    stack = @stack
    trap("ALRM") { __saruby__.timeout_fire(stack) }
    __saruby__.timeout_arm(stack)
    begin
      yield(sec)
    ensure
      @stack.delete_if { |e| e.equal?(ent) }
      __saruby__.timeout_arm(@stack)
    end
  end

  private def timeout(*args, &block)
    Timeout.timeout(*args, &block)
  end
end
