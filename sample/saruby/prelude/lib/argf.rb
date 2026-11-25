# ARGF: the files named in ARGV (or stdin) as one stream.  Loaded on first use.
# As in CRuby, ARGF's class is anonymous ("ARGF.class"); ARGF.class.new(*argv) reads its own argv.
# The state follows io.c's argf: @init_p (0 not started, 1 open, -1 closed by argf_close),
# @next_p (1 open the next name, 0 reading one, -1 reading stdin).
ARGF = Class.new(Object) {
  include Enumerable

  def self.name = "ARGF.class"
  def self.to_s = "ARGF.class"
  def self.inspect = "ARGF.class"

  def initialize(*argv)
    @argv = argv
    @init_p = 0
    @next_p = 0
    @lineno = 0
  end

  # argf_next_argv: false when there is no stream left
  def __next
    @init_p ||= 0
    @next_p ||= 0
    @lineno ||= 0
    if @init_p == 0
      @next_p = @argv && !@argv.empty? ? 1 : -1
      @init_p = 1
    elsif @argv.nil?
      @next_p = -1
    elsif @next_p == -1 && !@argv.empty?
      @next_p = 1
    end
    if @next_p == 1
      __close if @init_p == 1
      loop do
        if @argv.empty?
          @next_p = 1
          return false
        end
        name = @argv.shift
        name = name.to_path if !name.is_a?(String) && name.respond_to?(:to_path)
        name = String(name)
        @fname = name
        if name == "-"
          @io = $stdin
          if $-i
            warn "Can't do inplace edit for stdio; skipping"
            next
          end
        else
          @io = $-i ? __inplace_open(name) : File.open(name, "r")
          next unless @io
        end
        @io.binmode if @binmode
        @io.set_encoding(*@encs) if @encs && !@binmode
        @next_p = 0
        break
      end
    elsif @next_p == -1
      @io = $stdin
      @fname = "-"
      warn "Can't do inplace edit for stdio" if $-i
    end
    @init_p = 1 if @init_p == -1
    true
  end

  # -i: read the original (renamed to the backup, or unlinked) and write a new file through $stdout
  def __inplace_open(name)
    $stdout.close if $stdout.is_a?(File) && !$stdout.equal?(STDOUT) && !$stdout.closed?
    mode = File.stat(name).mode
    ext = $-i
    if ext.empty?
      r = File.open(name, "r")
      File.unlink(name)
    else
      bak = name + ext
      File.unlink(bak) rescue nil
      begin
        File.rename(name, bak)
      rescue SystemCallError => e
        warn "Can't rename #{name} to #{bak}: #{e.message.sub(/ @.*/, '')}, skipping file"
        return nil
      end
      r = File.open(bak, "r")
    end
    w = File.open(name, File::WRONLY | File::CREAT | File::TRUNC, 0666)
    w.chmod(mode)
    $stdout = w
    r
  end

  # argf_close
  def __close
    io = @io
    return if io.nil? || io.equal?($stdin)
    io.close unless io.closed?
    @init_p = -1
  end

  def __eof_next
    __close
    @next_p = 1
  end

  def gets(*args, **kw)
    loop do
      return nil unless __next
      line = @io.gets(*args, **kw)
      if line.nil? && @next_p != -1
        __eof_next
        next
      end
      if line
        @lineno += 1
        $. = @lineno
      end
      return line
    end
  end

  # ARGF_GENERIC_INPUT_P: the current stream is a $stdin that is not an IO (argf_forward sends it the call)
  def __generic = @io.equal?($stdin) && !@io.is_a?(IO)

  def readline(*args, **kw)
    raise EOFError, "end of file reached" unless __next
    return @io.public_send(:readline, *args, **kw) if __generic
    gets(*args, **kw) or raise EOFError, "end of file reached"
  end

  def each_line(*args, **kw, &blk)
    return to_enum(:each_line, *args, **kw) unless blk
    return @io.public_send(:each_line, *args, **kw, &blk) if __next && __generic
    while (line = gets(*args, **kw))
      yield line
    end
    self
  end
  alias each each_line

  def readlines(*args, **kw)
    return @io.public_send(:readlines, *args, **kw) if __next && __generic
    lines = []
    while (line = gets(*args, **kw))
      lines << line
    end
    lines
  end
  alias to_a readlines

  def read(length = nil, str = nil)
    len = length.nil? ? nil : Integer(length)
    if str
      str = String(str) unless str.is_a?(String)
      str.replace("")
    end
    arg = len
    loop do
      return str unless __next
      tmp = arg.nil? ? @io.read : @io.read(arg)
      if str.nil?
        str = tmp
      elsif tmp
        str << tmp
      end
      if tmp.nil? || length.nil?
        if @next_p != -1
          __eof_next
          next
        end
      elsif str.bytesize < len
        arg = len - str.bytesize
        next
      end
      return str
    end
  end

  # argf_getpartial
  def __partial(nonblock, len, buf, exception)
    unless __next
      buf&.replace("")
      raise EOFError, "end of file reached"
    end
    tmp = begin
      if nonblock
        buf ? @io.read_nonblock(len, buf, exception: exception) : @io.read_nonblock(len, exception: exception)
      else
        buf ? @io.readpartial(len, buf) : @io.readpartial(len)
      end
    rescue EOFError
      nil
    end
    return tmp unless tmp.nil?
    if @next_p != -1
      __eof_next
      return buf || +"" unless @argv.empty?
    end
    raise EOFError, "end of file reached" if exception
    nil
  end

  def readpartial(len, buf = nil) = __partial(false, len, buf, true)

  def read_nonblock(len, buf = nil, exception: true) = __partial(true, len, buf, exception)

  def getc
    loop do
      return nil unless __next
      ch = @io.getc
      if ch.nil? && @next_p != -1
        __eof_next
        next
      end
      return ch
    end
  end

  def getbyte
    loop do
      return nil unless __next
      b = @io.getbyte
      if b.nil? && @next_p != -1
        __eof_next
        next
      end
      return b
    end
  end

  def readchar
    loop do
      raise EOFError, "end of file reached" unless __next
      ch = @io.getc
      if ch.nil? && @next_p != -1
        __eof_next
        next
      end
      raise EOFError, "end of file reached" if ch.nil?
      return ch
    end
  end

  def readbyte
    b = getbyte
    raise EOFError, "end of file reached" if b.nil?
    b
  end

  def each_char
    return to_enum(:each_char) unless block_given?
    while (ch = getc)
      yield ch
    end
    self
  end

  def each_codepoint
    return to_enum(:each_codepoint) unless block_given?
    while (ch = getc)
      yield ch.ord
    end
    self
  end

  def each_byte
    return to_enum(:each_byte) unless block_given?
    while (b = getbyte)
      yield b
    end
    self
  end

  def eof?
    __next
    if @io
      return true if @init_p == 0
      __next
      return true if @io.eof?
    end
    false
  end
  alias eof eof?

  def filename
    __next
    @fname
  end
  alias path filename

  def file
    __next
    @io
  end

  def to_io
    __next
    @io.to_io
  end

  def fileno
    raise ArgumentError, "no stream" unless __next
    @io.fileno
  end
  alias to_i fileno

  def lineno = (@lineno || 0)
  def lineno=(n)
    @lineno = Integer(n)
    $. = @lineno
  end
  def argv = @argv

  def pos
    raise ArgumentError, "no stream to tell" unless __next
    @io.pos
  end
  alias tell pos

  def pos=(n)
    raise ArgumentError, "no stream to set position" unless __next
    @io.pos = n
  end

  def seek(*args)
    raise ArgumentError, "no stream to seek" unless __next
    @io.seek(*args)
  end

  def rewind
    raise ArgumentError, "no stream to rewind" unless __next
    @lineno = 0
    @io.rewind
  end

  def skip
    if @init_p && @init_p != 0 && @next_p == 0
      __close
      @next_p = 1
    end
    self
  end

  def close
    __next
    __close
    @next_p = 1 if @next_p != -1
    @lineno = 0
    self
  end

  def closed?
    __next
    @io.closed?
  end

  def binmode
    @binmode = true
    __next
    @io.binmode
    self
  end
  def binmode? = !!@binmode

  def external_encoding = @io ? @io.external_encoding : Encoding.default_external
  def internal_encoding = @io ? @io.internal_encoding : Encoding.default_internal

  def set_encoding(*args, **kw)
    raise ArgumentError, "no stream to set encoding" unless __next
    @io.set_encoding(*args, **kw)
    @encs = [@io.external_encoding, @io.internal_encoding].compact
    self
  end

  def inplace_mode = $-i&.dup
  def inplace_mode=(ext)
    $-i = ext.nil? || ext == false ? nil : String(ext)
  end

  def to_s = "ARGF"
  alias inspect to_s
  def to_write_io
    raise IOError, "not opened for writing" unless @io
    $stdout
  end
  def write(*a) = $stdout.write(*a)
  def print(*a) = $stdout.print(*a)
  def puts(*a) = $stdout.puts(*a)
  def printf(*a) = $stdout.printf(*a)
  def putc(c) = $stdout.putc(c)
}.allocate
ARGF.instance_variable_set(:@argv, ARGV)
