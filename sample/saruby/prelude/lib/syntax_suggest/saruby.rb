# saruby: what syntax_suggest (the default gem, loaded as is) reads from the prism gem and Ripper, defined under
# SyntaxSuggest so its unqualified `Prism` / `Ripper` find these and nothing changes outside it (as
# error_highlight/saruby.rb).  Prism.parse gives the errors of saruby's prism; Prism.lex_compat is the gem's own
# LexCompat (prism/lex_compat.rb, read into this namespace) over saruby's prism lexer (builtins/syntax_suggest.rs).
require "saruby/syntax_suggest"
require "delegate"

module SyntaxSuggest
  module Prism
    ParseError = Struct.new(:message)

    class ParseResult
      attr_reader :errors
      def initialize(errors) = @errors = errors
      def success? = errors.empty?
      def failure? = !success?
    end

    def self.parse(source, **) = ParseResult.new(__saruby__.ss_errors(source).map { ParseError.new(it) })

    class Result
      attr_reader :comments, :magic_comments, :data_loc, :errors, :warnings, :source

      def initialize(comments, magic_comments, data_loc, errors, warnings, source)
        @comments, @magic_comments, @data_loc, @errors, @warnings, @source = comments, magic_comments, data_loc, errors, warnings, source
      end
    end

    class LexResult < Result
      attr_reader :value

      def initialize(value, *rest)
        @value = value
        super(*rest)
      end
    end

    module Source
      def self.for(source, *) = source
    end

    Location = Struct.new(:start_line, :start_column, :start_offset, :end_offset) do
      include Comparable
      def <=>(other) = start_offset <=> other.start_offset
    end

    class Token
      attr_reader :type, :location
      def initialize(type, value, location) = (@type, @value, @location = type, value, location)
      def value = @value.dup
    end

    # Prism.lex: [[token, lex state], ...]
    def self.lex(source, line: 1, **)
      toks = __saruby__.ss_lex(source, line).map do |type, value, l, col, st, en, state|
        [Token.new(type, value, Location.new(l, col, st, en)), state]
      end
      LexResult.new(toks, [], [], nil, [], [], source)
    end

    def self.lex_compat(source, **options) = LexCompat.new(source, **options).result

    module Translation
      # prism's Ripper translation: the lexer state bits (ripper's values) and Lexer::State
      class Ripper
        LEX_STATE_NAMES = %i[BEG END ENDARG ENDFN ARG CMDARG MID FNAME DOT CLASS LABEL LABELED FITEM].map.with_index.to_h { |name, i| [2**i, name] }.freeze
        LEX_STATE_NAMES.each { |value, key| const_set("EXPR_#{key}", value) }
        EXPR_NONE = 0
        EXPR_VALUE = EXPR_BEG
        EXPR_BEG_ANY = EXPR_BEG | EXPR_MID | EXPR_CLASS
        EXPR_ARG_ANY = EXPR_ARG | EXPR_CMDARG
        EXPR_END_ANY = EXPR_END | EXPR_ENDARG | EXPR_ENDFN

        def self.lex_state_name(state) = LEX_STATE_NAMES.filter_map { |flag, name| name if state & flag != 0 }.join("|")

        class Lexer
          class State
            attr_reader :to_int, :to_s

            def initialize(i)
              @to_int = i
              @to_s = Ripper.lex_state_name(i)
              freeze
            end

            alias to_i to_int
            alias inspect to_s
            def ==(i) = super || to_int == i
            def &(i) = self.class.new(to_int & i)
            def |(i) = self.class.new(to_int | i)
            def allbits?(i) = to_int.allbits?(i)
            def anybits?(i) = to_int.anybits?(i)
            def nobits?(i) = to_int.nobits?(i)
          end
        end
      end
    end
  end

  Ripper = Prism::Translation::Ripper

  path = $LOAD_PATH.lazy.map { File.join(it.to_s, "prism/lex_compat.rb") }.find { File.file?(it) } or raise LoadError, "prism/lex_compat"
  module_eval(File.read(path), path)

  # core_ext.rb's SyntaxError#detailed_message, after its `super` (builtins/exception.rs)
  def __saruby__.ss_annotate(path, highlight, message)
    require "syntax_suggest/api" unless defined?(SyntaxSuggest::DEFAULT_VALUE)

    file = Pathname.new(path)
    io = SyntaxSuggest::MiniStringIO.new

    SyntaxSuggest.call(
      io: io,
      source: file.read,
      filename: file,
      terminal: highlight
    )
    annotation = io.string

    annotation += "\n" unless annotation.end_with?("\n")

    annotation + message
  rescue => e
    if ENV["SYNTAX_SUGGEST_DEBUG"]
      $stderr.warn(e.message)
      $stderr.warn(e.backtrace)
    end

    # Ignore internal errors
    message
  end
end
