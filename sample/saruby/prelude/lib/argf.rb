# ARGF: the files named in ARGV (or stdin) as one stream.  Loaded on first use.
# As in CRuby, ARGF's class is anonymous ("ARGF.class"); ARGF.class.new(*argv) reads its own argv.
ARGF = Class.new(Object) {
  include Enumerable

  def self.name = "ARGF.class"
  def self.to_s = "ARGF.class"
  def self.inspect = "ARGF.class"

  def initialize(*argv)
    @argv = argv
  end

  def __io
    return @io if @io
    unless @started
      @started = true
      @lineno ||= 0
      if @argv.empty?
        @fname = "-"
        return @io = $stdin
      end
    end
    return nil if @argv.empty?
    @fname = @argv.shift
    @io = @fname == "-" ? $stdin : File.open(@fname)
    @io.binmode if @binmode
    @io
  end

  def __advance
    return nil if @started && @argv.empty?   # the last file stays open at EOF
    @io.close if @io && !@io.equal?($stdin) && !@io.closed?
    @io = nil
    __io
  end

  def gets(*args, **kw)
    while (io = __io)
      line = io.gets(*args, **kw)
      if line
        @lineno += 1
        $. = @lineno
        return line
      end
      __advance or break
    end
    nil
  end

  def readline(*args, **kw)
    gets(*args, **kw) or raise EOFError, "end of file reached"
  end

  def each_line(*args, **kw)
    return to_enum(:each_line, *args, **kw) unless block_given?
    while (line = gets(*args, **kw))
      yield line
    end
    self
  end
  alias each each_line

  def readlines(*args, **kw)
    lines = []
    while (line = gets(*args, **kw))
      lines << line
    end
    lines
  end
  alias to_a readlines

  def read(length = nil, outbuf = nil)
    buf = +""
    while (io = __io)
      s = io.read(length && length - buf.bytesize)
      buf << s if s
      break if length && buf.bytesize >= length
      __advance or break
    end
    return nil if length && length > 0 && buf.empty?
    outbuf ? outbuf.replace(buf) : buf
  end

  def readpartial(*args)
    while (io = __io)
      begin
        return io.readpartial(*args)
      rescue EOFError
        __advance or raise
      end
    end
    raise EOFError, "end of file reached"
  end

  def getc
    while (io = __io)
      ch = io.getc
      return ch if ch
      __advance or break
    end
    nil
  end

  def getbyte
    while (io = __io)
      b = io.getbyte
      return b if b
      __advance or break
    end
    nil
  end

  def readchar = getc || raise(EOFError, "end of file reached")
  def readbyte = getbyte || raise(EOFError, "end of file reached")

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

  def read_nonblock(*args, **kw)
    while (io = __io)
      begin
        return io.read_nonblock(*args, **kw)
      rescue EOFError
        __advance or raise
      end
    end
    raise EOFError, "end of file reached"
  end

  def seek(*args) = file.seek(*args)

  def each_byte
    return to_enum(:each_byte) unless block_given?
    while (b = getbyte)
      yield b
    end
    self
  end

  def eof?
    io = __io or raise IOError, "ARGF is closed"
    io.eof?
  end
  alias eof eof?

  def filename
    __io
    @fname
  end
  alias path filename

  def file
    __io
    @io
  end
  alias to_io file

  def fileno = file.fileno
  alias to_i fileno
  def lineno = (@lineno || 0)
  def lineno=(n)
    @lineno = n
    $. = n
  end
  def argv = @argv
  def pos = file.pos
  alias tell pos
  def pos=(n)
    file.pos = n
  end
  def rewind
    @lineno = 0
    file.rewind
  end
  def skip
    __advance if @io
    self
  end
  def close
    @io.close if @io && !@io.equal?($stdin) && !@io.closed?
    @io = nil
    __io
    self
  end
  def closed?
    __io
    @io.nil? || @io.closed?
  end
  def binmode
    @binmode = true
    @io.binmode if @io
    self
  end
  def binmode? = !!@binmode
  def external_encoding = (file || $stdin).external_encoding
  def internal_encoding = (file || $stdin).internal_encoding
  def set_encoding(*args)
    file.set_encoding(*args)
    self
  end
  def to_s = "ARGF"
  alias inspect to_s
  def write(*a) = $stdout.write(*a)
  def print(*a) = $stdout.print(*a)
  def puts(*a) = $stdout.puts(*a)
  def printf(*a) = $stdout.printf(*a)
  def putc(c) = $stdout.putc(c)
  def inplace_mode = nil
}.allocate
ARGF.instance_variable_set(:@argv, ARGV)
