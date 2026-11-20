class C
  def initialize; @any = true; @hclk = 70; @x = 0; end
  def run
    i = 0
    while i < 3_000_000
      if @any
        @x += 1 if @hclk >= 64
      end
      i += 1
    end
    @x
  end
end
p C.new.run
