# saruby TODO

優先順位（2026-11-21 user 指示）: **1. ASTro 原則 → 2. unsafe → 3. CRuby 互換性**。性能はこの 3 つの下。
新しい作業を起こす前にこの順に照らす。計測は `~/ruby/src/trials/`、採否の記録は `~/ruby/src/ideas/`。

## 1. ASTro 原則

原則の出典: `~/ruby/astro/docs/perf.md` §0、`docs/idea.md`、`docs/idea_code_store.md`。
監査: `~/ruby/src/trials/2026-11-21-saruby-astro-principles/README.md`（Audit 1, 2）。

- [x] 行番号を構造ハッシュから外す（`@hole`、フレームは行への参照）。iseq / CallInfo / ヒープリテラルを内容でハッシュ
- [x] patch 束縛（初回ディスパッチで穴を即値に。koruby の既定に合わせ既定 ON、`SARUBY_PATCH=0` で無効）
- [x] 再利用テスト `tools/reuse_test.sh`（別プログラムで焼いたストアが効くこと）
- [x] 焼き込むプロファイル値をハッシュに混ぜる（ASTro の Hopt: SD_<Hopt>、`hopt_index.txt` に (Horg, ファイル・行・入口種別) → Hopt）。first bake wins は解消
  - [x] hopt_index のキー: ファイルパス → ソースの内容のハッシュ + 行 + 入口種別（別パス・コピーでも自分の Hopt。reuse_test.sh の copy ケース）
  - [x] 起動 +2M 命令の分解と削減（trials/2026-11-23-saruby-hopt-startup）: 索引の文字列パース + HashMap 0.94M、
    body ごとの format! キーと KEYS（String の HashMap）約 1.0M（--plain でも払っていた）、core.rb の FNV 0.12M。
    索引は 16 桁 hex 3 列の固定幅・Horg 順（SWAR で読む、二分探索）、キーは u64（hash_merge）、キーと Horg はストアが
    使うときだけ計算、prelude はソースをハッシュしない（ビルドごとのストア）。`-e 'p 1'` 20.20M → 17.97M（Hopt 前 17.54M）、
    --plain 15.47M → 13.98M。bake: fib の item 52 → 83（R_/S_ が entry ごとに複製）、optcarrot +2%（rustc 時間は誤差内）
  - [x] item の名前は中身だけで決まる（trials/2026-11-23-saruby-item-sharing）。R_ / S_ の tag が entry の Hopt
    だったので entry ごとに複製されていた（fib: R/S 各 4。T 40 → 65 はその R_ 名を含む T_、確認済み）→ 再帰先自身の
    構造ハッシュ + profile（rec_tag）。T_ / L_ は構造ハッシュ（ノードから読むだけのオペランド、def の名前や body も
    入る）と patch.o の位置（P<offset>、穴の無い部分木にも）で別名になっていた → コード本文のハッシュ。
    fib items 83 → 50 / pitems 119 → 81、optcarrot items 10261 → 7394 / pitems 18423 → 10837（all.rs −21%、
    patch.rs −30%）。同じコードで別名の item（dupcheck.rb）は全ベンチ・optcarrot で 0。命令数は全ベンチ同じ
  - [ ] 同じコードで別の R_ / S_ 名: 再帰先の profile のうち焼かれない部分（spec の深さ制限より先）が違うとき（未観測）
  - [ ] 残る起動の固定費: prelude の body ごとの dlsym 0.42M（Hopt 前から）、索引の読み込み 0.16M（callgrind）
  - ivar スロット番号（user: 「ノードのパラメータにして焼くだけなら原則を破らない」）
  - site kind、`shared`、組み込んだ呼び出し先・ブロック、演算子の型、イテレータの選択
- [x] 組み込みメソッドの展開を名前で選ぶ（F4）。size/length/empty?/to_i/to_int は +6〜20 命令/呼び出し
- [x] inline iterator / inline builtin を builtin 自身のループ・関数を呼ぶ形に（F2, F3。iterators −11% サイクル）
- [x] EVAL body の残りの手書き fast path を特殊化か parser-pass へ（Round 3）
  - [x] `node_block` / `node_ary` / 呼び出し引数の手展開 → 生成器の `EVAL_EACH!`（特化でリストを展開、AOT 命令数は base と同じ）
  - [x] `send` + リテラル Symbol → node_send_sym（compile.rs、Kernel#send ガード付き。u_send_lit 381 → 146 命令）
  - [x] `<<` `[]` `[]=` → builtin 自身の関数。凍結時のバックトレースが CRuby と一致。演算子ノードの型による近道は原則違反ではない（user 2026-11-22）: String#<< の同エンコーディング・非凍結は安全なアクセサで node に残した（str AOT 2831M → 2591M、round 3 前 2566M）。Hash#[]= の行の保存で hash +2%、Array#<< で ary +3% は残る
- [x] `yield_block` を 1 コピーに（interp hashiter +7.9% / mapreduce +2.1%、AOT は同じ）。lookup_method / new_object / invoke / raise_exc は 1 コピーにすると nesteddata +9% なので残す

## 2. unsafe

「メソッド実装で使っていなければだいぶいい」「unsafe 外しは性能と見合い」（user）。外す前にコストを測り、遅くなる所は数字を添えて残す。src/builtins/ は sys.rs（OS インターフェース）以外 `forbid(unsafe_code)` 済み。
crate 全体 364（2026-11-21 d3400bc）。分類と経緯は `docs/safe_heap.md`。

- [x] builtins（全波）、inline_builtin.rs / inline_iter.rs、string/array/hash/range/bignum/regexp 本体、binding.rs、boot.rs
- [x] コスト 0 で外せるものを入れた（unsafe 371 → 342）: 定数の Value を const fn、LitCache を Option<Value>、REFINEMENTS_ON・BOOTED・gc::STACK_BASE・STRESS を atomic、CallCache.cfunc を Option<fn>、aot::STORE を thread_local、焼いた即値を const { Value::imm(..) }。safe な Value::fix/sym/flonum は hash の regalloc を変えて +0.6〜1.4% のため見送り（unsafe_catalog.md 4.4）
- [x] 39 件の発見元を分類（借用検査 1 / 実行時検査 1 / 読み直し・テスト 37、unsafe_catalog.md）
- [x] `&'static mut` の穴: frame()・vm()（&'static Vm）・heap()（gc.rs 内）・オブジェクトのアクセサ全部（object.rs は &'static mut を返さない、旧アクセサ呼び出し 233 → 0）。残りは gc.rs 内部の heap() 1 つ。途中で {**h} ほか 5 か所の解放済みオブジェクト参照を修正（unsafe_catalog 行 40）
- [ ] sbuf.rs（String のバッファ、10 か所）: safe 版はブランチ `sbuf-safe-rs29`（str AOT +6% 命令 / +11.5% サイクル）。採否未定
- [ ] 呼び出し経路・フレーム連鎖（vm.rs 80、rt.rs 64、node.def 16、stack_raw.rs 5）。前提 H5（実行中の builtin の下のフレームは生きている）は未検証
- 残す（外部インターフェース・土台）: sys.rs、kalloc.rs、fiber_os.rs、patch ローダ、heap.rs / value.rs / gc.rs の土台
- [x] gc.rs の `heap()` も `&'static mut` を返さない: `heap(&mut Scope)`（一時値 Scope の文で借用が終わる。入れ子は debug ビルドで assert、全テスト 0 件）。fn->&'static mut 1 → 0、命令数・機械語とも base と同じ（trials/2026-11-22-saruby-static-mut「gc heap」）
  - with_heap の +23% の原因: クロージャ（alloc の fast path 全体）が AOT 側で inline されず、1 確保ごとに呼び出し + body のスタック経由コピー。`#[inline(always)]` を付けると gcchurn は戻るが、str は内側クロージャの捕獲変数のストアが空きリスト判定の前に出て node_str +3 命令/回（str interp +0.85%・AOT +1.4%）。クロージャ形は不採用
- [x] main ctx の root 走査が、確保側の生きている `&mut Ctx` と重なる: 確保は全部 `gc::alloc(c, ..)`（context-free の 90 か所を段階的に変換、NO_CTX・str_legacy.rs・legacy の生成関数を削除。unsafe 382 → 380、static mut 10 → 9）。main ctx の by-place 読みは fiber が確保するときだけ。master 比: 15 ベンチ中 10・起動・optcarrot は同じ、object AOT +0.62 %（焼いたコードのレジスタ割り当て）、nesteddata AOT +1.0 %（Hash の malloc 越しに c を保持）、hash AOT +1.4 %（hash_set のレジスタ割り当て。旧 base と同じ値で、無関係な cold 編集で反転）。途中の事故 2 件（drop_body の Box<RHash> drop glue が外に出る、RHash::with_capacity が inline されない）は修正（trials/2026-11-22-saruby-static-mut「NO_CTX」）
- [ ] vm 分割後の poly AOT +0.29%（mcache の RefCell 検査の推定、未検証）。sp4 の時間で要確認
- [x] 生成コード（astrustgen のテンプレート）: ノードアクセスを node_head.rs の監査済みプリミティブ（node_u::<T> ほか 13 個）に集約、@ref は NodeCell（UnsafeCell）、SD の子は wrapping_byte_offset。saruby 生成 1165 → 93、fib の焼き all.rs 75 → 7。命令数は全セル ±0.016% 以内（poly のみ -0.58%、call_slow の cgu 分割、cgu1 では 0）。L_::nth の unreachable_unchecked は safe 形が optcarrot .text +1% のため残す（unsafe_catalog.md 8）
- [ ] 生成コード残り: @ref の &mut は再入（再帰呼び出しサイト）で一意でない（base から同じ UB）。body に &NodeCell + Cell のキャッシュを渡す形は node.def / vm.rs のキャッシュ API の変更が要る。NodePtr を &'static Node / NonNull にする案は手書き側の変更が大きく未着手（unsafe_catalog.md 8.3）
- [ ] 見送り中: safe な Value::fix/sym/flonum（hash +0.6〜1.4%、regalloc の変化）、ConstCache.val の Option 化（+0.1〜0.3%）
- [ ] 再帰呼び出しの call site が、自分の inline cache（@ref）への &mut を、先の &mut が生きている間にもう 1 つ作る（base からの UB。NodeCell 化後も残る。node.def・vm.rs で cache の借用を呼び出しの前に終える必要）

## 3. CRuby 互換性

- [x] rubyspec を実 mspec で計測（runner: ~/ruby/src/trials/2026-11-22-saruby-rubyspec/bin/、SPEC_RUBY_VERSION=4.0.2）。core 19542/22373、language 2592/2933、library 1922/3188（CRuby 4.0.2: 23144/23155、2934/2934、6401/6402）
- [x] AOT で knucleotide が segfault → loader の Arena を fork 後に共有していた（3ea7ec4）
- [x] rubyspec 第 2 回（~/ruby/src/trials/2026-11-22-saruby-rubyspec-2/）: 特殊グローバル変数、IO の外部/内部エンコーディング（io.c 移植、読み書きの変換）、マジックコメントのソースエンコーディング、再帰構造の #hash、Range#step/#max、Kernel#Float、タイムアウト 10 ファイル。core 19617 → 20324、language 2592 → 2720
- [x] Encoding::Converter / String#encode を CRuby 4.0 の transcode.c から移植（econv.rs、表は enc/trans/*.c から tools/gen_trans_tables.rb で生成、9.5 MB）、TracePoint / set_trace_func（tracepoint.rs）。converter 142/142、tracepoint 74/76（~/ruby/src/trials/2026-11-22-saruby-converter-tracepoint/）
- [x] TracePoint の thread_begin / thread_end（fiber_main で発火、フックの例外はスレッドを終わらせる。tracepoint 74 → 76 / 76、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [ ] TracePoint の残り: トレース用にコンパイルされていないコードは line と演算子の c_call を出さない。CRuby が内部で呼ぶ Exception#initialize・Module#===（rescue）・const_added / inherited の c_call は出ない
- [ ] rubyspec の大きな塊: IO::Buffer 180
- [x] Regexp のエンコーディング（~/ruby/src/trials/2026-11-23-saruby-regexp/）: /e /s /u /n・FIXEDENCODING / NOENCODING・rb_reg_preprocess / prepare_enc 相当（astrogre に UTF-8 / 1 バイト / EUC-JP / Shift_JIS / 2 バイト系 / EUC-TW / GB18030 の読み方）、UTF-16/32 は UTF-8 形で照合、inspect / to_s / union、Regexp.timeout（実行時に強制）、try_convert・linear_time?・MatchData#match 等。core/regexp 164 → 252 / 256、matchdata 165 → 185 / 185、language/regexp 224 → 251 / 258
  - [ ] 残り: 旧来マルチバイトの文字に \p{..}、ISO-8859-x の文字種・大小文字、[[:punct:]] Ps 等の表、gsub / scan / split の素の探索は timeout 無し、CRuby がメモ化で線形にする指数バックトラック（`(a*)*$`）、eval した非 UTF-8 文字列のソースエンコーディング（regexp リテラルが UTF-8 扱い）
  - [ ] liquid-compile +0.47 % 命令（StringScanner#scan +42 / =~ +44 命令/呼び出し: 照合ごとのエンコーディング判定・timeout・モード引数）。String に coderange のキャッシュがあれば判定の走査が消える
- [x] IO の変換を変換エンジン経由に（io.c の readconv / cbuf / writeconv、builtins/io_conv.rs、docs/io_design.md「Encoding conversion」、~/ruby/src/trials/2026-11-23-saruby-io-transcode/）: 変換してから分ける（gets / getc / read / ungetc の cbuf、AFTER_OUTPUT、行の一般分岐を変換なしの IO にも）、newline: / xml: / replace: 装飾、ISO-2022-JP・UTF-16/32 への書き出し（close で終端）、byte 読みと char 読みの区別、BOM|、IO.pipe / popen / IO.read・write のエンコーディングとオプション、open_args:。CRuby との差分コーパス 28,412 件で base 16,669 → 28,343 一致（残りは sprintf の UTF-16 引数・GB18030 の limit 切り・CRuby の 8 KiB バッファ依存）、rubyspec core/io 1286 → 1325（他の 5 dir は増減なし）。gets 系は read_line の毎行コピーをやめて −62〜76 % 命令、io_pipe_rw −56 %
  - [ ] 残り: 変換ありの gets が遅い（econv の AFTER_OUTPUT 呼び出しが 1 文字ずつ: Shift_JIS→UTF-8 の gets が base（行ごとに変換、CRuby と違う結果）の 2.3 倍の命令）、GB18030 / Big5 / EUC-KR の limit 切り（Onigmo の left_adjust_char_head 未移植）、生の UTF-16 の limit 切りで CRuby の 8 KiB バッファ依存の挙動、File.open + close が +1.1 %（open ごと +43 命令: parse_mode と RIo の 4 欄）
  - [ ] IO#puts / #print / #p が上書きされた #write を呼ばない（rubyspec io/puts 14 件）
  - [x] UTF-16 / UTF-32 の String メソッド（chomp / chop / strip 系 / lines / 揃え / partition / 大小文字を UTF-8 形で）、Integer#chr(enc)、sprintf の非 ASCII 互換引数（~/ruby/src/trials/2026-11-23-saruby-compat-bundle/、差分調査 407 → 33 行）
  - [ ] 残り: 幅広文字列の to_sym（Symbol にエンコーディングが無い）、壊れた UTF-8 などの strip が CRuby の ArgumentError / CompatibilityError を出さない（coderange が無いので全走査が要る）
- [x] blop 層（docs/io_design.md、~/ruby/src/trials/2026-11-22-saruby-blop/）: 待ちは全部 slab 所有の op、エンジンは poll（既定）/ epoll。1 ms ポーリング廃止。file/flock・file/open・io-wait/wait・process/status/wait のタイムアウト解消
- [ ] まだタイムアウト: kernel/require（並行 require: ロード中 feature の待ちと他スレッドの Thread#backtrace が要る）、mutex/lock・thread/list（グリーンスレッドに横取りが無い: ループ後方枝 / メソッド入口のタイムスライス検査が要る）
- [ ] blop: epoll の登録を fd 単位で持ち越す、io_uring エンジン、CFUNC ワーカー（flock / FIFO open / waitpid(-1) のバックオフ代替、getaddrinfo）
- [ ] Queue ping-pong 1.7M → 1.0M round trips/s（blop の簿記）
- [x] `$~` / `$_` をメソッドのフレームローカルに（ブロックと共有、builtins/svar.rs: 呼び出しごとのコスト 0、`$~` を書く照合ごと +101〜151 命令）、`$?` はスレッドローカル、`$0 =` は /proc/self/comm（~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [ ] svar の残り: 生きているメソッドのフレームのブロックを別ファイバー / スレッドで呼ぶと `$~` を共有しない（id だけで引く）、死んだ Binding への eval の return（base と同じくプロセスが黙って終わる）、`b.binding.local_variable_set` した変数を eval から見えない、`$0 =` は CRuby と違い argv 領域（ps の COMMAND）でなく comm を変える
- [x] library/ 2496 → 6156 / 6376（CRuby 6401 / 6402、~/ruby/src/trials/2026-11-22-saruby-stdlib-2/）: digest、zlib（zlib 1.3 の deflate / inflate を移植、出力はバイト一致）、psych / yaml（libyaml 0.2.5 の移植）、bigdecimal（bigdecimal.c の移植）、socket（+ net-http / net-ftp / ipaddr / resolv）、StringIO（stringio.c 3.2.0 の移植に置き換え）、syslog、etc の sysconf / confstr、cgi/escape.so、coverage（lines / oneshot / methods / branches / eval）、rbconfig/sizeof
- [ ] library/ の残り: openssl 173（部分実装は `defined?(OpenSSL::SSL)` 判定を壊すので丸ごと）、objectspace 46（trace_object_allocations は割り当てフックが要る）、weakref の回収、`Coverage.line_stub`（RubyVM::InstructionSequence）
- [ ] 起動の命令数が codegen-units=16 の分割で ±0.8M 動く（`__rust_alloc` がどの呼び出し元に inline されるか）。比較は `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` で（base 16.177M → stdlib-2 16.190M。既定ビルドの先頭は json の FxHash 化で 15.91M）
- [ ] 値を使わない式（`if` の値など、外側から決まるもの）の line イベントがまだ CRuby より多い（文単位の省略はした）
- [ ] autoload が `main.require` を呼ばない（core/kernel/autoload_spec）、Enumerator#first を別スレッドから供給したときの break（enum_first_cross_thread_1022）
- [ ] RUBY_VERSION を 4.0.x にするか（いまは 3.4.0。mspec は SPEC_RUBY_VERSION で回避）。変わるもの（調査のみ、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/ README）: RbConfig の MAJOR/MINOR/ruby_version/RUBY_API_VERSION も揃える必要（Gem.default_dir が lib/ruby/gems/3.4.0 を指し 4.0.2 の gem が見えない）、4.0.2 の rbconfig.rb は 4.0.x 以外で raise、bundled_gems.rb の SINCE 判定（ostruct / logger / benchmark / pstore / irb 等が 3.4.0 では :warning、4.0 では :error 側）、gem の `require "cgi/util" if RUBY_VERSION < "3.5"`（3.4.0 だと 4.0 で消えた CGI::Util を読みに行く）、prism ffi の version 3、Bundler の ruby_40?、RUBY_DESCRIPTION の "Ruby 3.4 compatible"
- [x] CRuby benchmark/*.rb 148/148（yml は 228/236: IO::Buffer、RubyVM、irb、Pathname、fiber_chain）
- [ ] ruby-bench 67/88（check、plain。stdlib-2 で zlib・psych・json/ext が入り chunky-png・hexapdf・psych-load・liquid-*・json_parse_* が一致）。残り: Ractor 3 本、未インストール gem 18 本
- [x] `Date` / `DateTime`（prelude/lib/date.rb、ext/date の Ruby 移植）、`Pathname`（CRuby 4.0 同様に組み込み、遅延ロード）、`require 'time'` は CRuby の time.rb
- [x] Fiber storage、`Fiber#blocking?`、fiber-local `Thread#[]`、`Fiber#inspect` の状態、fiber called across threads
- [ ] Fiber scheduler、`Fiber#raise(cause:)`
- [x] Enumerator がキーワード引数を渡さない・private メソッドを呼べない（rb_block_call と同じに、stdlib-2）
- [ ] case/when が再定義された `Integer#===` を無視する（再定義フラグ 32bit が満杯 → 64bit に）
- [x] 再帰的な正規表現サブ式呼び出し `(?<a>a\g<a>?)`・`\g<0>`・レベル付き後方参照 `\k<a+0>`・absent 演算子 `(?~x)`（2026-11-23-saruby-regexp）
- [x] blop 層（docs/io_design.md）: 待ちを 1 か所に、slab + Id + 所有権ごと渡すバッファ、engine trait（poll 既定・epoll）。待機中の CPU ほぼ 0、外部 cat との往復 1k → 45k/s。rubyspec file/flock・file/open・io-wait/wait・process/status/wait のタイムアウト解消
- [ ] blop の残り: Mutex/CV/Queue の受け渡しが重い（vm_thread_condvar2 +59%、condvar1 +28%、sized_queue4 +26% 命令）、epoll の登録キャッシュ、io_uring engine、worker thread（flock・FIFO open・Process.wait(-1) は back-off の仮実装）
- [ ] 時分割がない: ブロックしないスレッドが他を飢えさせる（rubyspec mutex/lock・thread/list がタイムアウト）。node.def に時分割のチェックが要る（ASTro 原則と相談: 無効時コスト 0 の形で）
- [ ] `Array#shift` が O(n)（so_lists）。O(1) 版 2 案は他の配列アクセスを遅くして却下済み
- [ ] 大きな左シフト（CRuby は NoMemoryError、saruby は 2^26 bit 超を RangeError）
- [x] `__callee__` on alias、`WeakMap#inspect`、`detailed_message` / `full_message` の書式（eval_error.c / error.c の移植、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [ ] error_highlight（CRuby の未捕捉例外に出るソースの ^^^ 表示）が無い
- [ ] t/hand: 約 23,400 PASS / 10,200 FAIL
- [ ] 解説書『saruby でわかる Rust』（~/ruby/src/trials/2026-11-21-saruby-rust-book）: vm/heap 分割の反映、trait と Ruby の module の対比・dispatch（Dyn と T_… の Disp）の説明を追加（2026-11-22 の会話）
- [x] `Array#cycle(n).size` が i64 で `len * n` してあふれる: Bignum で返す（CRuby master と同じ。4.0.2 は LONG2FIX で折り返す）。同時に Enumerable.__int の Fixnum 折り返しと Enumerator#size が Bignum を nil にするのも修正
- [ ] `cycle` の size が即時計算: `[1].cycle(2**64)` が作った時点で RangeError（CRuby は size を呼ぶまで遅延して Enumerator を返す）
- [ ] `$0 =` / setproctitle: CRuby は argv 領域を書き換える（ps のコマンド行）。saruby は何もしない（comm を変えるのは CRuby と逆の振る舞いなので外した）。Rust が持たないメモリへの書き込みが要る

## 性能（上 3 つの後。新規には起こさない）
- [ ] NO_CTX 除去の代償: hash +1.4%（hash_set の regalloc、コード配置がもろい）、nesteddata +1.0%（Hash の malloc をまたいで c を保持）、object +0.62%（AOT 命令数）
- [ ] `$~` を設定する正規表現マッチ 1 回 +101〜151 命令（frame-local 化、1M マッチで +5〜10%）。メソッド起動ごとにフレーム探索をキャッシュすれば大半が消える見込み
- [ ] IO 変換 round 1 の代償: io_file_create（File.open+close）+1.07%（open 1 回 ~43 命令）、io_select2/3 +0.33/+0.23%、変換付き gets 2.3×・puts 1.6×（io.c 同様に 1 文字ずつ受け渡すため）
- [ ] svar の代償: `=~` 1 回 +101〜151 命令（rb_t / rb_m / rb_b で +4.9 / +5.0 / +10.4 %、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）。家の解決を活性化ごとにキャッシュすれば大半が消える
- [ ] 正規表現 round 1 の代償: liquid-compile +0.47%・liquid-render +0.16%（マッチごとのエンコーディング判定 ~44 命令、timeout 参照、ステップ計数）。String の coderange キャッシュで判定を省ける
- [ ] rclass 除去（a1217f5）の代償を取り戻す: poly +0.85%（interp）/ +1.17%（AOT）、object +0.3%、optcarrot +0.36%/フレーム（命令数）。原因は call_slow のキャッシュ hit 経路で c を引き回すレジスタ移動 ~2.7 命令/呼び出し、ivar 読みの miss 呼び出しに c を渡す 1 命令、new_object のクラスの ivar 数読み 1 命令（user 2026-11-22「高速化は別途考えましょう」）

- [x] sp4 の退行 ary: glibc がメモリをカーネルに返して再フォルト → `M_TOP_PAD` 64 MiB（ary 110 → 68 ms）
- [x] sp4 の退行 ackermann: patch 束縛ではなく all.so と patch.o で引数リストの名前付けが違い別の木を比べていた → 名前付けを揃えた（a1246fd）
- [ ] method_call/send の 3dd858b 退行（Zen 4）: 1 呼び出しあたりストアが 1 つ多い、出所未特定（Zen 3 の memory disambiguation とは別）
- [ ] bake を SD 単位にコンパイル・キャッシュ（いまはプログラムごとに all.rs 4.6 MB + patch.rs 7.9 MB を 1 crate で。optcarrot 47 秒、30k_* は rustc がメモリ不足）
- [x] 長いベンチ（`bench_long/`、本体約 1 秒）での比較を artifact に（long.html）
- [ ] koruby との差（起動差し引き、sp4）: tak・gcd・fib・ackermann（呼び出し 1 回の命令数）、bitops・intdiv（slow path 合流後の再検査）、optcarrot の L1d
- [ ] 演算子ノードの型別の近道は、外れたときに型判定を順に重ねる分だけ高い → PGO で対処（user 2026-11-22）。`node_opt_ltlt` には他の演算子ノード（plus/aref/aset）にある型の記録 `tf@ref: TyFb` + `ty@bake` が無いので付け、焼いたコードは観測した型だけを先に判定（値は Hopt に入る）
- [ ] 受け手をレジスタに保持（method_call −7% / optcarrot +0.4% 命令、判断待ち）
- [ ] 起動: prelude の def 本体の遅延コンパイル（残り 13.7M 命令のうち 6.8M）
- [ ] 起動の後戻り: `-e 'p 1'` 13.7M（de58efd）→ 20.6M 命令（2026-11-22、hopt_index・Pathname の起動時定義ほか）。`require 'date'` は +13 ms/プロセス
- [ ] ivar の多相: 親クラスのメソッド内の `@a` が親・サブクラス両方のオブジェクトから使われると並びが違い、単相のキャッシュが外れる（shape 的な仕組み）。スロット番号は実行しないと決まらない（継承・条件分岐・instance_variable_set）ので静的割り当てはしない（2026-11-22 user と確認）
- [ ] sort の AOT サイクルが alloc-sweep の 2・3 番目のコミット後に +1〜3% かも（局所機では判定不能、sp4 で再計測）
- [ ] 生成器 astrustgen.rb を build.rs に移す（ideas/idea/astrust-gen-in-rust.md）。ASTrust が成熟してから（user 2026-11-22）

## 計測の約束

- 平均（geomean 等）は出さない。ベンチごとの実測値とばらつき
- 起動時間は `_startup` 行で分離。0.1 秒級のベンチは起動で律速するので長いベンチを使う
- sp4 は bench-lease。ローカル（Zen 3、共有）は命令数のみ信頼
