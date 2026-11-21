# frozen_string_literal: true
# Pathname: CRuby 4.0 pathname_builtin.rb (pathname 0.4.0, by Tanaka Akira; BSD-2-Clause / Ruby
# license), documentation comments removed; Kernel#Pathname is in core.rb. pathname.c's
# Pathname#<=> and #sub are not ported: the file's own Ruby definitions of them are used.

class Pathname

  VERSION = "0.4.0"

  if File::FNM_SYSCASE.nonzero?
    private def same_paths?(a, b) a.casecmp(b) == 0 end
  else
    private def same_paths?(a, b) a == b end
  end

  attr_reader :path
  protected :path

  def initialize(path)
    @path = File.path(path).dup
  rescue TypeError => e
    raise e.class, "Pathname.new requires a String, #to_path or #to_str", cause: nil
  end

  def freeze
    super
    @path.freeze
    self
  end

  def ==(other)
    return false unless Pathname === other
    other.path == @path
  end
  alias === ==
  alias eql? ==

  unless method_defined?(:<=>, false)
    def <=>(other)
      return nil unless Pathname === other
      @path.tr('/', "\0") <=> other.path.tr('/', "\0")
    end
  end

  def hash # :nodoc:
    @path.hash
  end

  def to_s
    @path.dup
  end

  alias to_path to_s

  def inspect # :nodoc:
    "#<#{self.class}:#{@path}>"
  end

  unless method_defined?(:sub, false)
    def sub(pattern, *args, **kwargs, &block)
      if block
        path = @path.sub(pattern, *args, **kwargs) {|*sub_args|
          begin
            old = Thread.current[:pathname_sub_matchdata]
            Thread.current[:pathname_sub_matchdata] = $~
            eval("$~ = Thread.current[:pathname_sub_matchdata]", block.binding)
          ensure
            Thread.current[:pathname_sub_matchdata] = old
          end
          yield(*sub_args)
        }
      else
        path = @path.sub(pattern, *args, **kwargs)
      end
      self.class.new(path)
    end
  end

  def sub_ext(repl)
    ext = File.extname(@path)

    unless @path.end_with?(ext)
      ext = @path[@path.rindex(ext)..]
    end

    self.class.new(@path.chomp(ext) + repl)
  end

  if File::ALT_SEPARATOR
    SEPARATOR_LIST = Regexp.quote "#{File::ALT_SEPARATOR}#{File::SEPARATOR}"
    SEPARATOR_PAT = /[#{SEPARATOR_LIST}]/
  else
    SEPARATOR_LIST = Regexp.quote File::SEPARATOR
    SEPARATOR_PAT = /#{SEPARATOR_LIST}/
  end
  SEPARATOR_LIST.freeze
  SEPARATOR_PAT.freeze
  private_constant :SEPARATOR_LIST, :SEPARATOR_LIST

  if File.dirname('A:') == 'A:.' # DOSish drive letter
    ABSOLUTE_PATH = /\A(?:[A-Za-z]:|#{SEPARATOR_PAT})/
  else
    ABSOLUTE_PATH = /\A#{SEPARATOR_PAT}/
  end
  ABSOLUTE_PATH.freeze
  private_constant :ABSOLUTE_PATH

  def mkpath(mode: nil)
    path = @path == '/' ? @path : @path.chomp('/')

    stack = []
    until File.directory?(path) || File.dirname(path) == path
      stack.push path
      path = File.dirname(path)
    end

    stack.reverse_each do |dir|
      dir = dir == '/' ? dir : dir.chomp('/')
      if mode
        Dir.mkdir dir, mode
        File.chmod mode, dir
      else
        Dir.mkdir dir
      end
    rescue SystemCallError
      raise unless File.directory?(dir)
    end

    self
  end

  def chop_basename(path) # :nodoc:
    base = File.basename(path)
    if /\A#{SEPARATOR_PAT}?\z/o.match?(base)
      return nil
    else
      return path[0, path.rindex(base)], base
    end
  end
  private :chop_basename

  def split_names(path) # :nodoc:
    names = []
    while r = chop_basename(path)
      path, basename = r
      names.unshift basename
    end
    return path, names
  end
  private :split_names

  def prepend_prefix(prefix, relpath) # :nodoc:
    if relpath.empty?
      File.dirname(prefix)
    elsif SEPARATOR_PAT.match?(prefix)
      prefix = File.dirname(prefix)
      prefix = File.join(prefix, "") if File.basename(prefix + 'a') != 'a'
      prefix + relpath
    else
      prefix + relpath
    end
  end
  private :prepend_prefix

  def cleanpath(consider_symlink=false)
    if consider_symlink
      cleanpath_conservative
    else
      cleanpath_aggressive
    end
  end

  def cleanpath_aggressive # :nodoc:
    path = @path
    names = []
    pre = path
    while r = chop_basename(pre)
      pre, base = r
      case base
      when '.'
      when '..'
        names.unshift base
      else
        if names[0] == '..'
          names.shift
        else
          names.unshift base
        end
      end
    end
    pre.tr!(File::ALT_SEPARATOR, File::SEPARATOR) if File::ALT_SEPARATOR
    if SEPARATOR_PAT.match?(File.basename(pre))
      names.shift while names[0] == '..'
    end
    self.class.new(prepend_prefix(pre, File.join(*names)))
  end
  private :cleanpath_aggressive

  def has_trailing_separator?(path) # :nodoc:
    if r = chop_basename(path)
      pre, basename = r
      pre.length + basename.length < path.length
    else
      false
    end
  end
  private :has_trailing_separator?

  def add_trailing_separator(path) # :nodoc:
    if File.basename(path + 'a') == 'a'
      path
    else
      File.join(path, "") # xxx: Is File.join is appropriate to add separator?
    end
  end
  private :add_trailing_separator

  def del_trailing_separator(path) # :nodoc:
    if r = chop_basename(path)
      pre, basename = r
      pre + basename
    elsif /#{SEPARATOR_PAT}+\z/o =~ path
      $` + File.dirname(path)[/#{SEPARATOR_PAT}*\z/o]
    else
      path
    end
  end
  private :del_trailing_separator

  def cleanpath_conservative # :nodoc:
    path = @path
    names = []
    pre = path
    while r = chop_basename(pre)
      pre, base = r
      names.unshift base if base != '.'
    end
    pre.tr!(File::ALT_SEPARATOR, File::SEPARATOR) if File::ALT_SEPARATOR
    if SEPARATOR_PAT.match?(File.basename(pre))
      names.shift while names[0] == '..'
    end
    if names.empty?
      self.class.new(File.dirname(pre))
    else
      if names.last != '..' && File.basename(path) == '.'
        names << '.'
      end
      result = prepend_prefix(pre, File.join(*names))
      if /\A(?:\.|\.\.)\z/ !~ names.last && has_trailing_separator?(path)
        self.class.new(add_trailing_separator(result))
      else
        self.class.new(result)
      end
    end
  end
  private :cleanpath_conservative

  def parent
    self + '..'
  end

  def mountpoint?
    begin
      stat1 = self.lstat
      stat2 = self.parent.lstat
      stat1.dev != stat2.dev || stat1.ino == stat2.ino
    rescue Errno::ENOENT
      false
    end
  end

  def root?
    chop_basename(@path) == nil && SEPARATOR_PAT.match?(@path)
  end

  def absolute?
    ABSOLUTE_PATH.match? @path
  end

  def relative?
    !absolute?
  end

  def each_filename # :yield: filename
    return to_enum(__method__) unless block_given?
    _, names = split_names(@path)
    names.each {|filename| yield filename }
    nil
  end

  def descend
    return to_enum(__method__) unless block_given?
    vs = []
    ascend {|v| vs << v }
    vs.reverse_each {|v| yield v }
    nil
  end

  def ascend
    return to_enum(__method__) unless block_given?
    path = @path
    yield self
    while r = chop_basename(path)
      path, = r
      break if path.empty?
      yield self.class.new(del_trailing_separator(path))
    end
  end

  def +(other)
    other = Pathname.new(other) unless Pathname === other
    Pathname.new(plus(@path, other.path))
  end
  alias / +

  def plus(path1, path2) # -> path # :nodoc:
    prefix2 = path2
    index_list2 = []
    basename_list2 = []
    while r2 = chop_basename(prefix2)
      prefix2, basename2 = r2
      index_list2.unshift prefix2.length
      basename_list2.unshift basename2
    end
    return path2 if prefix2 != ''
    prefix1 = path1
    while true
      while !basename_list2.empty? && basename_list2.first == '.'
        index_list2.shift
        basename_list2.shift
      end
      break unless r1 = chop_basename(prefix1)
      prefix1, basename1 = r1
      next if basename1 == '.'
      if basename1 == '..' || basename_list2.empty? || basename_list2.first != '..'
        prefix1 = prefix1 + basename1
        break
      end
      index_list2.shift
      basename_list2.shift
    end
    r1 = chop_basename(prefix1)
    if !r1 && (r1 = SEPARATOR_PAT.match?(File.basename(prefix1)))
      while !basename_list2.empty? && basename_list2.first == '..'
        index_list2.shift
        basename_list2.shift
      end
    end
    if !basename_list2.empty?
      suffix2 = path2[index_list2.first..-1]
      r1 ? File.join(prefix1, suffix2) : prefix1 + suffix2
    else
      r1 ? prefix1 : File.dirname(prefix1)
    end
  end
  private :plus

  def join(*args)
    return self if args.empty?
    result = args.pop
    result = Pathname.new(result) unless Pathname === result
    return result if result.absolute?
    args.reverse_each {|arg|
      arg = Pathname.new(arg) unless Pathname === arg
      result = arg + result
      return result if result.absolute?
    }
    self + result
  end

  def children(with_directory=true)
    with_directory = false if @path == '.'
    result = []
    Dir.foreach(@path) {|e|
      next if e == '.' || e == '..'
      if with_directory
        result << self.class.new(File.join(@path, e))
      else
        result << self.class.new(e)
      end
    }
    result
  end

  def each_child(with_directory=true, &b)
    children(with_directory).each(&b)
  end

  def relative_path_from(base_directory)
    base_directory = Pathname.new(base_directory) unless base_directory.is_a? Pathname
    dest_directory = self.cleanpath.path
    base_directory = base_directory.cleanpath.path
    dest_prefix = dest_directory
    dest_names = []
    while r = chop_basename(dest_prefix)
      dest_prefix, basename = r
      dest_names.unshift basename if basename != '.'
    end
    base_prefix = base_directory
    base_names = []
    while r = chop_basename(base_prefix)
      base_prefix, basename = r
      base_names.unshift basename if basename != '.'
    end
    unless same_paths?(dest_prefix, base_prefix)
      raise ArgumentError, "different prefix: #{dest_prefix.inspect} and #{base_directory.inspect}"
    end
    while !dest_names.empty? &&
          !base_names.empty? &&
          same_paths?(dest_names.first, base_names.first)
      dest_names.shift
      base_names.shift
    end
    if base_names.include? '..'
      raise ArgumentError, "base_directory has ..: #{base_directory.inspect}"
    end
    base_names.fill('..')
    relpath_names = base_names + dest_names
    if relpath_names.empty?
      Pathname.new('.')
    else
      Pathname.new(File.join(*relpath_names))
    end
  end
end

class Pathname    # * File *
  def each_line(...) # :yield: line
    File.foreach(@path, ...)
  end

  def read(...) File.read(@path, ...) end

  def binread(...) File.binread(@path, ...) end

  def readlines(...) File.readlines(@path, ...) end

  def sysopen(...) File.sysopen(@path, ...) end

  def write(...) File.write(@path, ...) end

  def binwrite(...) File.binwrite(@path, ...) end

  def atime() File.atime(@path) end

  def birthtime() File.birthtime(@path) end

  def ctime() File.ctime(@path) end

  def mtime() File.mtime(@path) end

  def chmod(mode) File.chmod(mode, @path) end

  def lchmod(mode) File.lchmod(mode, @path) end

  def chown(owner, group) File.chown(owner, group, @path) end

  def lchown(owner, group) File.lchown(owner, group, @path) end

  def fnmatch(pattern, ...) File.fnmatch(pattern, @path, ...) end

  def fnmatch?(pattern, ...) File.fnmatch?(pattern, @path, ...) end

  def ftype() File.ftype(@path) end

  def make_link(old) File.link(old, @path) end

  def open(...) # :yield: file
    File.open(@path, ...)
  end

  def readlink() self.class.new(File.readlink(@path)) end

  def rename(to) File.rename(@path, to) end

  def stat() File.stat(@path) end

  def lstat() File.lstat(@path) end

  def make_symlink(old) File.symlink(old, @path) end

  def truncate(length) File.truncate(@path, length) end

  def utime(atime, mtime) File.utime(atime, mtime, @path) end

  def lutime(atime, mtime) File.lutime(atime, mtime, @path) end

  def basename(...) self.class.new(File.basename(@path, ...)) end

  def dirname() self.class.new(File.dirname(@path)) end

  def extname() File.extname(@path) end

  def expand_path(...) self.class.new(File.expand_path(@path, ...)) end

  def split()
    array = File.split(@path)
    raise TypeError, 'wrong argument type nil (expected Array)' unless Array === array
    array.map {|f| self.class.new(f) }
  end

  def realpath(...) self.class.new(File.realpath(@path, ...)) end

  def realdirpath(...) self.class.new(File.realdirpath(@path, ...)) end
end

class Pathname    # * FileTest *

  def blockdev?() FileTest.blockdev?(@path) end

  def chardev?() FileTest.chardev?(@path) end

  def empty?
    if FileTest.directory?(@path)
      Dir.empty?(@path)
    else
      File.empty?(@path)
    end
  end

  def executable?() FileTest.executable?(@path) end

  def executable_real?() FileTest.executable_real?(@path) end

  def exist?() FileTest.exist?(@path) end

  def grpowned?() FileTest.grpowned?(@path) end

  def directory?() FileTest.directory?(@path) end

  def file?() FileTest.file?(@path) end

  def pipe?() FileTest.pipe?(@path) end

  def socket?() FileTest.socket?(@path) end

  def owned?() FileTest.owned?(@path) end

  def readable?() FileTest.readable?(@path) end

  def world_readable?() File.world_readable?(@path) end

  def readable_real?() FileTest.readable_real?(@path) end

  def setuid?() FileTest.setuid?(@path) end

  def setgid?() FileTest.setgid?(@path) end

  def size() FileTest.size(@path) end

  def size?() FileTest.size?(@path) end

  def sticky?() FileTest.sticky?(@path) end

  def symlink?() FileTest.symlink?(@path) end

  def writable?() FileTest.writable?(@path) end

  def world_writable?() File.world_writable?(@path) end

  def writable_real?() FileTest.writable_real?(@path) end

  def zero?() FileTest.zero?(@path) end
end

class Pathname    # * Dir *
  def Pathname.glob(*args, **kwargs) # :yield: pathname
    if block_given?
      Dir.glob(*args, **kwargs) {|f| yield self.new(f) }
    else
      Dir.glob(*args, **kwargs).map {|f| self.new(f) }
    end
  end

  def glob(*args, **kwargs) # :yield: pathname
    if block_given?
      Dir.glob(*args, **kwargs, base: @path) {|f| yield self + f }
    else
      Dir.glob(*args, **kwargs, base: @path).map {|f| self + f }
    end
  end

  def Pathname.getwd() self.new(Dir.getwd) end
  class << self
    alias pwd getwd
  end

  def entries() Dir.entries(@path).map {|f| self.class.new(f) } end

  def each_entry(&block) # :yield: pathname
    return to_enum(__method__) unless block_given?
    Dir.foreach(@path) {|f| yield self.class.new(f) }
  end

  def mkdir(...) Dir.mkdir(@path, ...) end

  def rmdir() Dir.rmdir(@path) end

  def opendir(&block) # :yield: dir
    Dir.open(@path, &block)
  end
end

class Pathname    # * mixed *
  def unlink()
    Dir.unlink @path
  rescue Errno::ENOTDIR
    File.unlink @path
  end
  alias delete unlink
end

class Pathname
  undef =~ if Kernel.method_defined?(:=~)
end

