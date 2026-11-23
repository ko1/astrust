# Process::Tms / Waiter / UID / GID / Sys (builtins/process.rs: lazy constants, loaded on first reference).
module Process
  Tms = Struct.new(:utime, :stime, :cutime, :cstime)

  # Process.detach's thread
  class Waiter < Thread
    def self.__start(pid)
      t = new(pid) { |q| Process.__detach_wait(q) }
      t[:pid] = pid
      t
    end
    def pid = self[:pid]
    # CRuby's watcher thread has no Ruby block: no location in #inspect
    def inspect = super.sub(%r{ <internal:saruby/process>:\d+}, "")
    alias to_s inspect
  end

  module UID
    def self.rid = Process.uid
    def self.eid = Process.euid
    def self.from_name(name) = Process.__id_of(name, false)
    def self.grant_privilege(id)
      id = Process.__id_of(id, false)
      Process.__setresid(false, -1, id, Process.uid == id ? -1 : id)
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      id = Process.__id_of(id, false)
      Process.__setresid(false, id, id, id)
      id
    end
    def self.re_exchange
      r, e, = Process.__resid(false)
      Process.__setresid(false, e, r, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
  end

  module GID
    def self.rid = Process.gid
    def self.eid = Process.egid
    def self.from_name(name) = Process.__id_of(name, true)
    def self.grant_privilege(id)
      id = Process.__id_of(id, true)
      Process.__setresid(true, -1, id, Process.gid == id ? -1 : id)
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      id = Process.__id_of(id, true)
      Process.__setresid(true, id, id, id)
      id
    end
    def self.re_exchange
      r, e, = Process.__resid(true)
      Process.__setresid(true, e, r, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
  end

  module Sys
    def self.getuid = Process.uid
    def self.geteuid = Process.euid
    def self.getgid = Process.gid
    def self.getegid = Process.egid
    def self.seteuid(id) = Process.__setresid(false, -1, id, -1)
    def self.setegid(id) = Process.__setresid(true, -1, id, -1)
    def self.setresuid(r, e, s) = Process.__setresid(false, r, e, s)
    def self.setresgid(r, e, s) = Process.__setresid(true, r, e, s)
    def self.issetugid = raise(NotImplementedError, "issetugid() function is unimplemented on this machine")
  end
end
