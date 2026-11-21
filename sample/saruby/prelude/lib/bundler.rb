# A minimal Bundler: Bundler.setup puts the lib dirs of the gems locked in
# Gemfile.lock on $LOAD_PATH.  Nothing is resolved or installed; the gems must
# already be installed for the reference ruby (`bundle check` passes).
module Bundler
  class BundlerError < StandardError; end
  class GemNotFound < BundlerError; end
  class GemfileNotFound < BundlerError; end

  class UI
    attr_accessor :level
    def initialize; @level = :info; end
    def warn(msg) = (Kernel.warn(msg) unless @level == :silent)
    def info(msg) = nil
    def error(msg) = Kernel.warn(msg)
    def debug(msg) = nil
    def silence = yield
  end

  class << self
    def ui = (@ui ||= UI.new)
    attr_writer :ui

    def default_gemfile
      if (g = ENV["BUNDLE_GEMFILE"]) && !g.empty?
        return File.expand_path(g)
      end
      dir = Dir.pwd
      loop do
        %w[Gemfile gems.rb].each { |n| f = File.join(dir, n); return f if File.file?(f) }
        up = File.dirname(dir)
        raise GemfileNotFound, "Could not locate Gemfile" if up == dir
        dir = up
      end
    end

    def default_lockfile
      g = default_gemfile
      g.end_with?("gems.rb") ? g.sub(/gems\.rb\z/, "gems.locked") : "#{g}.lock"
    end

    def root = File.dirname(default_gemfile)

    def gem_dirs
      dirs = []
      dirs << ENV["GEM_HOME"] if ENV["GEM_HOME"] && !ENV["GEM_HOME"].empty?
      dirs.concat(ENV["GEM_PATH"].split(File::PATH_SEPARATOR)) if ENV["GEM_PATH"] && !ENV["GEM_PATH"].empty?
      lib = RbConfig::CONFIG["rubylibdir"]
      if lib && !lib.empty?
        ver = File.basename(lib)
        dirs << File.join(File.dirname(lib), "gems", ver)
        dirs << File.join(Dir.home, ".gem", "ruby", ver) rescue nil
      end
      dirs.uniq.select { |d| File.directory?(d) }
    end

    # [[name, version, kind, source]] in lockfile order
    def locked_specs(lockfile)
      specs = []
      section = nil; source = {}; in_specs = false
      File.foreach(lockfile) do |line|
        line = line.chomp
        if line =~ /\A([A-Z]+)\z/
          section = $1; source = {}; in_specs = false
        elsif line =~ /\A  (remote|revision|ref|branch|glob): (.*)\z/
          source[$1] = $2
        elsif line == "  specs:"
          in_specs = true
        elsif in_specs && %w[GEM PATH GIT].include?(section) && line =~ /\A    ([^ ]+) \(([^)]+)\)\z/
          specs << [$1, $2, section, source.dup]
        end
      end
      specs
    end

    def require_paths(gemspec_file)
      return ["lib"] unless gemspec_file && File.file?(gemspec_file)
      src = File.read(gemspec_file)
      if src =~ /\.require_paths\s*=\s*\[([^\]]*)\]/
        paths = $1.scan(/"([^"]*)"|'([^']*)'/).map { |a, b| a || b }
        return paths unless paths.empty?
      end
      ["lib"]
    end

    def find_installed(name, version)
      gem_dirs.each do |gd|
        cands = [version] + %w[x86_64-linux x86_64-linux-gnu ruby].map { |pl| "#{version}-#{pl}" }
        cands.each do |v|
          dir = File.join(gd, "gems", "#{name}-#{v}")
          next unless File.directory?(dir)
          spec = File.join(gd, "specifications", "#{name}-#{v}.gemspec")
          return [dir, spec]
        end
        spec = File.join(gd, "specifications", "default", "#{name}-#{version}.gemspec")
        return [nil, spec] if File.file?(spec) # a default gem: already in rubylibdir
      end
      nil
    end

    def setup(*_groups)
      return self if @setup_done
      lockfile = default_lockfile
      raise GemfileNotFound, "#{lockfile} not found" unless File.file?(lockfile)
      base = File.dirname(lockfile)
      paths = []
      locked_specs(lockfile).each do |name, version, kind, source|
        next if name == "bundler"
        version = version.sub(/-(x86_64|aarch64|arm64|java|universal)[-\w]*\z/, "")
        case kind
        when "PATH"
          dir = File.expand_path(source["remote"] || ".", base)
          spec = Dir.glob(File.join(dir, "#{name}.gemspec")).first
          paths.concat(require_paths(spec).map { |p| File.expand_path(p, dir) })
        when "GIT"
          rev = source["revision"].to_s[0, 12]
          dir = gem_dirs.map { |gd| File.join(gd, "bundler", "gems", "#{name}-#{rev}") }.find { |d| File.directory?(d) }
          dir ||= gem_dirs.flat_map { |gd| Dir.glob(File.join(gd, "bundler", "gems", "*-#{rev}")) }.first
          raise GemNotFound, "Could not find #{name}-#{version} (git #{rev})" unless dir
          spec = Dir.glob(File.join(dir, "{,*/}#{name}.gemspec")).first
          sdir = spec ? File.dirname(spec) : dir
          paths.concat(require_paths(spec).map { |p| File.expand_path(p, sdir) })
        else
          found = find_installed(name, version)
          raise GemNotFound, "Could not find #{name}-#{version} in locally installed gems" unless found
          dir, spec = found
          next unless dir
          paths.concat(require_paths(spec).map { |p| File.expand_path(p, dir) })
        end
      end
      paths.reverse_each { |p| $LOAD_PATH.unshift(p) unless $LOAD_PATH.include?(p) }
      @setup_done = true
      self
    end

    def require(*groups)
      setup(*groups)
    end
  end
end

# What the harness and gems expect of RubyGems; saruby has none.
module Gem
  LoadError = Class.new(::LoadError) unless const_defined?(:LoadError, false)
  MissingSpecError = Class.new(Gem::LoadError) unless const_defined?(:MissingSpecError, false)
  autoload :Deprecate, "rubygems/deprecate"
  autoload :Version, "rubygems/version"
  autoload :Requirement, "rubygems/requirement"
  module BUNDLED_GEMS
    def self.force_activate(name)
      raise Gem::MissingSpecError, "can't activate #{name}: saruby has no RubyGems"
    end
  end unless const_defined?(:BUNDLED_GEMS, false)
end
