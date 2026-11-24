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
    # rb_seteuid_core
    def self.__seteuid(id)
      Process.__chk(false)
      if Process.uid != id
        Process.__setresid(false, -1, id, id)
        Process.__set_saved(false, id)
      else
        Process.__setresid(false, -1, id, -1)
      end
      id
    end
    private_class_method :__seteuid
    def self.grant_privilege(id)
      __seteuid(Process.__id_of(id, false))
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      Process.__chk(false)
      n = Process.__id_of(id, false)
      if Process.euid == 0
        Process.__setresid(false, n, n, n)
      else
        k = ->(cur) { cur == n ? -1 : n }
        Process.__setresid(false, k.(Process.uid), k.(Process.euid), k.(Process.__saved(false)))
      end
      Process.__set_saved(false, n)
      id
    end
    def self.re_exchange
      Process.__chk(false)
      r = Process.uid
      e = Process.euid
      Process.__setresid(false, e, r, r)
      Process.__set_saved(false, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
    def self.switch
      Process.__chk(false)
      r = Process.uid
      e = Process.euid
      if r != e
        Process.euid= r
        back = Process.__saved(false)
        ret = e
      elsif e != (back = Process.__saved(false))
        Process.euid= back
        back = e
        ret = r
      else
        raise Errno::EPERM
      end
      return ret unless block_given?
      Process.__under(false, true)
      begin
        yield
      ensure
        Process.__under(false, false)
        __seteuid(back)
      end
    end
  end

  module GID
    def self.rid = Process.gid
    def self.eid = Process.egid
    def self.from_name(name) = Process.__id_of(name, true)
    # rb_seteuid_core
    def self.__seteuid(id)
      Process.__chk(true)
      if Process.gid != id
        Process.__setresid(true, -1, id, id)
        Process.__set_saved(true, id)
      else
        Process.__setresid(true, -1, id, -1)
      end
      id
    end
    private_class_method :__seteuid
    def self.grant_privilege(id)
      __seteuid(Process.__id_of(id, true))
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      Process.__chk(true)
      n = Process.__id_of(id, true)
      if Process.euid == 0
        Process.__setresid(true, n, n, n)
      else
        k = ->(cur) { cur == n ? -1 : n }
        Process.__setresid(true, k.(Process.gid), k.(Process.egid), k.(Process.__saved(true)))
      end
      Process.__set_saved(true, n)
      id
    end
    def self.re_exchange
      Process.__chk(true)
      r = Process.gid
      e = Process.egid
      Process.__setresid(true, e, r, r)
      Process.__set_saved(true, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
    def self.switch
      Process.__chk(true)
      r = Process.gid
      e = Process.egid
      if r != e
        Process.egid= r
        back = Process.__saved(true)
        ret = e
      elsif e != (back = Process.__saved(true))
        Process.egid= back
        back = e
        ret = r
      else
        raise Errno::EPERM
      end
      return ret unless block_given?
      Process.__under(true, true)
      begin
        yield
      ensure
        Process.__under(true, false)
        __seteuid(back)
      end
    end
  end

  module Sys
    def self.getuid = Process.uid
    def self.geteuid = Process.euid
    def self.getgid = Process.gid
    def self.getegid = Process.egid
    def self.setuid(id) = Process.__setid(0, id, nil)
    def self.setgid(id) = Process.__setid(1, id, nil)
    def self.setreuid(r, e) = Process.__setid(2, r, e)
    def self.setregid(r, e) = Process.__setid(3, r, e)
    def self.seteuid(id) = Process.__setresid(false, -1, id, -1)
    def self.setegid(id) = Process.__setresid(true, -1, id, -1)
    def self.setresuid(r, e, s) = Process.__setresid(false, r, e, s)
    def self.setresgid(r, e, s) = Process.__setresid(true, r, e, s)
    # glibc has no setruid / setrgid / issetugid
    def self.setruid(id) = raise(NotImplementedError, "setruid() function is unimplemented on this machine")
    def self.setrgid(id) = raise(NotImplementedError, "setrgid() function is unimplemented on this machine")
    def self.issetugid = raise(NotImplementedError, "issetugid() function is unimplemented on this machine")
  end
end
