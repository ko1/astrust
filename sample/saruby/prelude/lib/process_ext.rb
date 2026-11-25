# Process::Tms / Waiter / UID / GID / Sys (builtins/process.rs: lazy constants, loaded on first reference).
module Process
  Tms = Struct.new(:utime, :stime, :cutime, :cstime)

  # Process.detach's thread
  class Waiter < Thread
    def pid = self[:pid]
    # CRuby's watcher thread has no Ruby block: no location in #inspect
    def inspect = super.sub(%r{ <internal:saruby/process>:\d+}, "")
    alias to_s inspect
  end

  module UID
    def self.rid = Process.uid
    def self.eid = Process.euid
    def self.from_name(name) = __saruby__.__id_of(name, false)
    def self.grant_privilege(id)
      __saruby__.__seteuid(false, __saruby__.__id_of(id, false))
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      __saruby__.__chk(false)
      n = __saruby__.__id_of(id, false)
      if Process.euid == 0
        __saruby__.__setresid(false, n, n, n)
      else
        k = ->(cur) { cur == n ? -1 : n }
        __saruby__.__setresid(false, k.(Process.uid), k.(Process.euid), k.(__saruby__.__saved(false)))
      end
      __saruby__.__set_saved(false, n)
      id
    end
    def self.re_exchange
      __saruby__.__chk(false)
      r = Process.uid
      e = Process.euid
      __saruby__.__setresid(false, e, r, r)
      __saruby__.__set_saved(false, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
    def self.switch
      __saruby__.__chk(false)
      r = Process.uid
      e = Process.euid
      if r != e
        Process.euid= r
        back = __saruby__.__saved(false)
        ret = e
      elsif e != (back = __saruby__.__saved(false))
        Process.euid= back
        back = e
        ret = r
      else
        raise Errno::EPERM
      end
      return ret unless block_given?
      __saruby__.__under(false, true)
      begin
        yield
      ensure
        __saruby__.__under(false, false)
        __saruby__.__seteuid(false, back)
      end
    end
  end

  module GID
    def self.rid = Process.gid
    def self.eid = Process.egid
    def self.from_name(name) = __saruby__.__id_of(name, true)
    def self.grant_privilege(id)
      __saruby__.__seteuid(true, __saruby__.__id_of(id, true))
      id
    end
    def self.eid=(id)
      grant_privilege(id)
    end
    def self.change_privilege(id)
      __saruby__.__chk(true)
      n = __saruby__.__id_of(id, true)
      if Process.euid == 0
        __saruby__.__setresid(true, n, n, n)
      else
        k = ->(cur) { cur == n ? -1 : n }
        __saruby__.__setresid(true, k.(Process.gid), k.(Process.egid), k.(__saruby__.__saved(true)))
      end
      __saruby__.__set_saved(true, n)
      id
    end
    def self.re_exchange
      __saruby__.__chk(true)
      r = Process.gid
      e = Process.egid
      __saruby__.__setresid(true, e, r, r)
      __saruby__.__set_saved(true, r)
      r
    end
    def self.re_exchangeable? = true
    def self.sid_available? = true
    def self.switch
      __saruby__.__chk(true)
      r = Process.gid
      e = Process.egid
      if r != e
        Process.egid= r
        back = __saruby__.__saved(true)
        ret = e
      elsif e != (back = __saruby__.__saved(true))
        Process.egid= back
        back = e
        ret = r
      else
        raise Errno::EPERM
      end
      return ret unless block_given?
      __saruby__.__under(true, true)
      begin
        yield
      ensure
        __saruby__.__under(true, false)
        __saruby__.__seteuid(true, back)
      end
    end
  end

  module Sys
    def self.getuid = Process.uid
    def self.geteuid = Process.euid
    def self.getgid = Process.gid
    def self.getegid = Process.egid
    def self.setuid(id) = __saruby__.__setid(0, id, nil)
    def self.setgid(id) = __saruby__.__setid(1, id, nil)
    def self.setreuid(r, e) = __saruby__.__setid(2, r, e)
    def self.setregid(r, e) = __saruby__.__setid(3, r, e)
    def self.seteuid(id) = __saruby__.__setresid(false, -1, id, -1)
    def self.setegid(id) = __saruby__.__setresid(true, -1, id, -1)
    def self.setresuid(r, e, s) = __saruby__.__setresid(false, r, e, s)
    def self.setresgid(r, e, s) = __saruby__.__setresid(true, r, e, s)
    # glibc has no setruid / setrgid / issetugid
    def self.setruid(id) = raise(NotImplementedError, "setruid() function is unimplemented on this machine")
    def self.setrgid(id) = raise(NotImplementedError, "setrgid() function is unimplemented on this machine")
    def self.issetugid = raise(NotImplementedError, "issetugid() function is unimplemented on this machine")
  end
end

# Process.detach's thread (process.rs p_detach)
def __saruby__.__waiter_start(pid)
  t = Process::Waiter.new(pid) { |q| __detach_wait(q) }
  t[:pid] = pid
  t
end
# rb_seteuid_core (gid: Process::GID's)
def __saruby__.__seteuid(gid, id)
  __chk(gid)
  if (gid ? Process.gid : Process.uid) != id
    __setresid(gid, -1, id, id)
    __set_saved(gid, id)
  else
    __setresid(gid, -1, id, -1)
  end
  id
end
