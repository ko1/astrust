# frozen_string_literal: true
# CRuby 4.0.2 lib/pathname.rb (by Tanaka Akira; BSD-2-Clause / Ruby license), documentation
# comments removed: Pathname#find, #rmtree and Pathname.mktmpdir.

class Pathname    # * Find *
  def find(ignore_error: true) # :yield: pathname
    return to_enum(__method__, ignore_error: ignore_error) unless block_given?
    require 'find'
    if @path == '.'
      Find.find(@path, ignore_error: ignore_error) {|f| yield self.class.new(f.delete_prefix('./')) }
    else
      Find.find(@path, ignore_error: ignore_error) {|f| yield self.class.new(f) }
    end
  end
end

class Pathname    # * FileUtils *
  def rmtree(noop: nil, verbose: nil, secure: nil)
    require 'fileutils'
    FileUtils.rm_rf(@path, noop: noop, verbose: verbose, secure: secure)
    self
  end
end

class Pathname    # * tmpdir *
  def self.mktmpdir
    require 'tmpdir' unless defined?(Dir.mktmpdir)
    if block_given?
      Dir.mktmpdir do |dir|
        dir = self.new(dir)
        yield dir
      end
    else
      self.new(Dir.mktmpdir)
    end
  end
end
