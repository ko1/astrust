class Monitor
  def new_cond = ::MonitorMixin::ConditionVariable.new(self)
  alias try_mon_enter try_enter
  alias mon_try_enter try_enter
  alias mon_enter enter
  alias mon_exit exit
end

module MonitorMixin
  class ConditionVariable
    def initialize(monitor)
      @monitor = monitor
      @cond = Thread::ConditionVariable.new
    end

    def wait(timeout = nil)
      @monitor.mon_check_owner
      @monitor.wait_for_cond(@cond, timeout)
    end

    def wait_while
      wait while yield
    end

    def wait_until
      wait until yield
    end

    def signal
      @monitor.mon_check_owner
      @cond.signal
    end

    def broadcast
      @monitor.mon_check_owner
      @cond.broadcast
    end
  end

  def self.extend_object(obj)
    super(obj)
    obj.__send__(:mon_initialize)
  end

  def mon_try_enter = @mon_data.try_enter
  alias try_mon_enter mon_try_enter
  def mon_enter = @mon_data.enter
  def mon_exit
    mon_check_owner
    @mon_data.exit
  end
  def mon_locked? = @mon_data.mon_locked?
  def mon_owned? = @mon_data.mon_owned?
  def mon_synchronize(&b) = @mon_data.synchronize(&b)
  alias synchronize mon_synchronize
  def new_cond = ::MonitorMixin::ConditionVariable.new(@mon_data)

  def initialize(...)
    super
    mon_initialize
  end

  def mon_initialize
    @mon_data = ::Monitor.new
    @mon_data_owner_object_id = self.object_id
  end

  def mon_check_owner = @mon_data.mon_check_owner
end
