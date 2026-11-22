# Ruby port of syslog 0.3.0's ext/syslog/syslog.c (Copyright (C) 2001 Amos Gouaux, 2003 Akinori MUSHA;
# BSD-2-Clause / Ruby license). The libc calls are saruby natives (src/builtins/syslog.rs).
require "saruby/syslog"

module Syslog
  module Constants; end
  module Option
    { LOG_PID: 1, LOG_CONS: 2, LOG_ODELAY: 4, LOG_NDELAY: 8, LOG_NOWAIT: 16, LOG_PERROR: 32 }.each { |k, v| const_set(k, v) }
  end
  module Facility
    { LOG_AUTH: 32, LOG_AUTHPRIV: 80, LOG_CRON: 72, LOG_DAEMON: 24, LOG_FTP: 88, LOG_KERN: 0, LOG_LPR: 48, LOG_MAIL: 16,
      LOG_NEWS: 56, LOG_SYSLOG: 40, LOG_USER: 8, LOG_UUCP: 64, LOG_LOCAL0: 128, LOG_LOCAL1: 136, LOG_LOCAL2: 144,
      LOG_LOCAL3: 152, LOG_LOCAL4: 160, LOG_LOCAL5: 168, LOG_LOCAL6: 176, LOG_LOCAL7: 184 }.each { |k, v| const_set(k, v) }
  end
  module Level; end
  module Macros; end
  private_class_method :__openlog, :__syslog, :__closelog, :__setlogmask

  st = { opened: false, ident: nil, options: -1, facility: -1, mask: -1 }
  int = lambda do |v|   # NUM2INT
    return v if Integer === v
    return v.to_i if Float === v
    raise TypeError, "no implicit conversion from nil to integer" if v.nil?
    raise TypeError, "no implicit conversion of #{v == true || v == false ? v.inspect : v.class} into Integer" unless v.respond_to?(:to_int)
    v.to_int
  end
  write = lambda do |pri, args|
    raise ArgumentError, "no log message supplied" if args.size < 1
    raise RuntimeError, "must open syslog before write" unless st[:opened]
    __syslog(pri, format(*args))
  end

  define_method(:close) do
    raise RuntimeError, "syslog not opened" unless st[:opened]
    Syslog.__send__(:__closelog)
    st[:ident] = nil; st[:options] = st[:facility] = st[:mask] = -1; st[:opened] = false
    nil
  end
  define_method(:open) do |ident = nil, opt = nil, fac = nil, &blk|
    raise RuntimeError, "syslog already open" if st[:opened]
    ident = $0 if ident.nil?
    ident = String.try_convert(ident) or raise TypeError, "no implicit conversion of #{ident == true ? "true" : ident == false ? "false" : ident.class} into String"
    options = opt.nil? ? 3 : int.(opt)
    facility = fac.nil? ? 8 : int.(fac)
    st[:mask] = Syslog.__send__(:__openlog, ident, options, facility)
    st[:ident] = ident.dup; st[:options] = options; st[:facility] = facility; st[:opened] = true
    Syslog.__send__(:__setlogmask, st[:mask])
    if blk
      begin
        blk.(Syslog)
      ensure
        Syslog.close
      end
    end
    Syslog
  end
  define_method(:reopen) { |*args, &blk| Syslog.close; Syslog.open(*args, &blk) }
  alias_method :open!, :reopen
  define_method(:opened?) { st[:opened] }
  define_method(:ident) { st[:opened] ? st[:ident].b : nil }
  define_method(:options) { st[:opened] ? st[:options] : nil }
  define_method(:facility) { st[:opened] ? st[:facility] : nil }
  define_method(:log) do |*args|
    raise ArgumentError, "wrong number of arguments (given #{args.size}, expected 2+)" if args.size < 2
    pri = args.shift
    raise TypeError, "type mismatch: #{pri.class} given" unless Integer === pri && pri.bit_length < 63
    write.(pri, args)
    Syslog
  end
  define_method(:mask) { st[:opened] ? st[:mask] : nil }
  define_method(:mask=) do |m|
    raise RuntimeError, "must open syslog before setting log mask" unless st[:opened]
    Syslog.__send__(:__setlogmask, st[:mask] = int.(m))
    m
  end
  define_method(:instance) { Syslog }
  { LOG_EMERG: :emerg, LOG_ALERT: :alert, LOG_CRIT: :crit, LOG_ERR: :err, LOG_WARNING: :warning,
    LOG_NOTICE: :notice, LOG_INFO: :info, LOG_DEBUG: :debug }.each_with_index do |(c, m), pri|
    Level.const_set(c, pri)
    define_method(m) { |*args| write.(pri, args); Syslog }
  end
  module_function :open, :reopen, :opened?, :ident, :options, :facility, :log, :close, :mask, :mask=, :instance,
    :emerg, :alert, :crit, :err, :warning, :notice, :info, :debug
  module_function :open!
  singleton_class.alias_method :open!, :reopen
  define_singleton_method(:inspect) do
    next "<#Syslog: opened=false>" unless st[:opened]
    "<#Syslog: opened=true, ident=\"#{st[:ident]}\", options=#{st[:options]}, facility=#{st[:facility]}, mask=#{st[:mask]}>"
  end
  VERSION = "0.3.0"

  Macros.define_method(:LOG_MASK) { |pri| 1 << int.(pri) }
  Macros.define_method(:LOG_UPTO) { |pri| (1 << (int.(pri) + 1)) - 1 }
  module Macros
    def self.included(target) = (target.extend(Macros); self)
  end
  module Constants
    include Option
    include Facility
    include Level
    include Macros
    def self.included(target) = (target.extend(Macros); self)
  end
  include Constants
end
