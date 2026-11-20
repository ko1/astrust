module Etc
  VERSION = "1.4.6"
  Passwd = Struct.new(:name, :passwd, :uid, :gid, :gecos, :dir, :shell)
  Group = Struct.new(:name, :passwd, :gid, :mem)

  class << self
    def __passwd
      File.readlines("/etc/passwd", chomp: true).filter_map do |l|
        f = l.split(":", -1)
        next if f.size < 7 || l.start_with?("#")
        Passwd.new(f[0], f[1], f[2].to_i, f[3].to_i, f[4], f[5], f[6])
      end
    rescue SystemCallError
      []
    end

    def __group
      File.readlines("/etc/group", chomp: true).filter_map do |l|
        f = l.split(":", -1)
        next if f.size < 4 || l.start_with?("#")
        Group.new(f[0], f[1], f[2].to_i, f[3].split(","))
      end
    rescue SystemCallError
      []
    end

    def passwd
      list = __passwd
      return list.each { |e| yield e } if block_given?
      @pwi = (@pwi || 0)
      e = list[@pwi]
      @pwi += 1 if e
      e
    end
    alias getpwent passwd
    def setpwent = (@pwi = 0; nil)
    def endpwent = (@pwi = 0; nil)

    def group
      list = __group
      return list.each { |e| yield e } if block_given?
      @gri = (@gri || 0)
      e = list[@gri]
      @gri += 1 if e
      e
    end
    alias getgrent group
    def setgrent = (@gri = 0; nil)
    def endgrent = (@gri = 0; nil)

    def getpwuid(uid = Process.uid)
      uid = Integer(uid)
      __passwd.find { |e| e.uid == uid } or raise ArgumentError, "can't find user for #{uid}"
    end

    def getpwnam(name)
      name = String(name)
      __passwd.find { |e| e.name == name } or raise ArgumentError, "can't find user for #{name}"
    end

    def getgrgid(gid = Process.gid)
      gid = Integer(gid)
      __group.find { |e| e.gid == gid } or raise ArgumentError, "can't find group for #{gid}"
    end

    def getgrnam(name)
      name = String(name)
      __group.find { |e| e.name == name } or raise ArgumentError, "can't find group for #{name}"
    end

    def getlogin
      ENV["LOGNAME"] || ENV["USER"] || (getpwuid.name rescue nil)
    end

    def systmpdir = "/tmp"
    def sysconfdir = "/etc"

    def nprocessors
      n = (File.read("/proc/self/status")[/^Cpus_allowed:\s*([0-9a-f,]+)/, 1].delete(",").to_i(16).to_s(2).count("1") rescue 0)
      n > 0 ? n : 1
    end

    def uname
      u = File.read("/proc/sys/kernel/ostype").chomp rescue "Linux"
      { sysname: u,
        nodename: (File.read("/proc/sys/kernel/hostname").chomp rescue "localhost"),
        release: (File.read("/proc/sys/kernel/osrelease").chomp rescue ""),
        version: (File.read("/proc/sys/kernel/version").chomp rescue ""),
        machine: RbConfig::CONFIG["host_cpu"] }
    end
  end
end
