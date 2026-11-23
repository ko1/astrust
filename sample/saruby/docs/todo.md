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
- [ ] 生成コード残り: NodePtr を &'static Node / NonNull にする案は手書き側の変更が大きく未着手（unsafe_catalog.md 8.3）
- [ ] 見送り中: safe な Value::fix/sym/flonum（hash +0.6〜1.4%、regalloc の変化）、ConstCache.val の Option 化（+0.1〜0.3%）
- [x] 再入した node の inline cache（@ref）への &mut の二重化: @ref は T のまま置き body に &T、キャッシュは全部 Cell フィールド（OpCc・poly 表は OnceCell<Box>）。NodeCell 削除。再入は t/hand 119 ファイル（call/call_blk/yield/call_x/opt_ltlt/opt_eq/opt_aref/call_kw/send_sym/iasgn）。実害あり: base の AOT で call_spec_out が呼び出し前の cc.klass を再利用し A に B#f を実行（-Zmutable-noalias=no で正常、tools/aot/ic_reentry_spec_klass_1123.rb）。命令数: block AOT +3.65%（焼いた SD 1 個のレジスタ割り当て、ループ変種 4 本は ±0.005%）以外は同等か減（ivar AOT -2.6%）。optcarrot -0.07%（unsafe_catalog.md 8.4、trials/2026-11-23-saruby-ic-borrow）
- [ ] block AOT +3.65%（SD_f634 のループで &blk.info.line がスタック再読込、1 命令/step）: 原因はレジスタ割り当てで意味的コストではないが未解消

## 3. CRuby 互換性

- [x] rubyspec を実 mspec で計測（runner: ~/ruby/src/trials/2026-11-22-saruby-rubyspec/bin/、SPEC_RUBY_VERSION=4.0.2）。core 19542/22373、language 2592/2933、library 1922/3188（CRuby 4.0.2: 23144/23155、2934/2934、6401/6402）
- [x] AOT で knucleotide が segfault → loader の Arena を fork 後に共有していた（3ea7ec4）
- [x] rubyspec 第 2 回（~/ruby/src/trials/2026-11-22-saruby-rubyspec-2/）: 特殊グローバル変数、IO の外部/内部エンコーディング（io.c 移植、読み書きの変換）、マジックコメントのソースエンコーディング、再帰構造の #hash、Range#step/#max、Kernel#Float、タイムアウト 10 ファイル。core 19617 → 20324、language 2592 → 2720
- [x] Encoding::Converter / String#encode を CRuby 4.0 の transcode.c から移植（econv.rs、表は enc/trans/*.c から tools/gen_trans_tables.rb で生成、9.5 MB）、TracePoint / set_trace_func（tracepoint.rs）。converter 142/142、tracepoint 74/76（~/ruby/src/trials/2026-11-22-saruby-converter-tracepoint/）
- [x] TracePoint の thread_begin / thread_end（fiber_main で発火、フックの例外はスレッドを終わらせる。tracepoint 74 → 76 / 76、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [ ] TracePoint の残り: トレース用にコンパイルされていないコードは line と演算子の c_call を出さない。CRuby が内部で呼ぶ Exception#initialize・Module#===（rescue）・const_added / inherited の c_call は出ない
- [x] IO::Buffer（io_buffer.c 4.0.2 の移植、builtins/io_buffer.rs は forbid、docs/io_design.md「IO::Buffer」、~/ruby/src/trials/2026-11-23-saruby-io-buffer/）: rubyspec core/io/buffer 0 → 198 / 198（CRuby 4.0.2 も 198 / 198）。バイトは Vec / sys::Mapping（munmap on drop）/ String 自身、スライスはアドレス範囲を元の現在のメモリで検証して bounds-checked に取る（アドレスは比較だけ）。CRuby との差分スクリプト 7,300 行一致。4.0.2 が解放済みメモリ・範囲外を触る 6 ケースは例外にした（deviations.rb）。sys.rs +5 unsafe（mmap / mremap / slice 2 / munmap）。str / hash / nesteddata の命令数は同じ、起動（cgu1）は +1〜3k 命令（.so が大きくなった分の ld.so・maps 読み。走るコードの差ではない）
  - [x] ロックした String への `<<` が node_opt_ltlt の近道で通る → FL_FROZEN | FL_TMPLOCK を 1 回の検査に（str_modify と同じ、~/ruby/src/trials/2026-11-23-saruby-redef-flags/）
  - [ ] IO.pipe の fd が CRuby（O_NONBLOCK）と違いブロッキング: IO::Buffer#read が CRuby では -EAGAIN を返す所で待つ
- [x] Regexp のエンコーディング（~/ruby/src/trials/2026-11-23-saruby-regexp/）: /e /s /u /n・FIXEDENCODING / NOENCODING・rb_reg_preprocess / prepare_enc 相当（astrogre に UTF-8 / 1 バイト / EUC-JP / Shift_JIS / 2 バイト系 / EUC-TW / GB18030 の読み方）、UTF-16/32 は UTF-8 形で照合、inspect / to_s / union、Regexp.timeout（実行時に強制）、try_convert・linear_time?・MatchData#match 等。core/regexp 164 → 252 / 256、matchdata 165 → 185 / 185、language/regexp 224 → 251 / 258
  - [ ] 残り: 旧来マルチバイトの文字に \p{..}、ISO-8859-x の文字種・大小文字、[[:punct:]] Ps 等の表、gsub / scan / split の素の探索は timeout 無し、CRuby がメモ化で線形にする指数バックトラック（`(a*)*$`）、eval した非 UTF-8 文字列のソースエンコーディング（regexp リテラルが UTF-8 扱い）
  - [ ] liquid-compile +0.47 % 命令（StringScanner#scan +42 / =~ +44 命令/呼び出し: 照合ごとのエンコーディング判定・timeout・モード引数）。String に coderange のキャッシュがあれば判定の走査が消える
- [x] IO の変換を変換エンジン経由に（io.c の readconv / cbuf / writeconv、builtins/io_conv.rs、docs/io_design.md「Encoding conversion」、~/ruby/src/trials/2026-11-23-saruby-io-transcode/）: 変換してから分ける（gets / getc / read / ungetc の cbuf、AFTER_OUTPUT、行の一般分岐を変換なしの IO にも）、newline: / xml: / replace: 装飾、ISO-2022-JP・UTF-16/32 への書き出し（close で終端）、byte 読みと char 読みの区別、BOM|、IO.pipe / popen / IO.read・write のエンコーディングとオプション、open_args:。CRuby との差分コーパス 28,412 件で base 16,669 → 28,343 一致（残りは sprintf の UTF-16 引数・GB18030 の limit 切り・CRuby の 8 KiB バッファ依存）、rubyspec core/io 1286 → 1325（他の 5 dir は増減なし）。gets 系は read_line の毎行コピーをやめて −62〜76 % 命令、io_pipe_rw −56 %
  - [ ] 残り: 変換ありの gets が遅い（econv の AFTER_OUTPUT 呼び出しが 1 文字ずつ: Shift_JIS→UTF-8 の gets が base（行ごとに変換、CRuby と違う結果）の 2.3 倍の命令）、GB18030 / Big5 / EUC-KR の limit 切り（Onigmo の left_adjust_char_head 未移植）、生の UTF-16 の limit 切りで CRuby の 8 KiB バッファ依存の挙動、File.open + close が +1.1 %（open ごと +43 命令: parse_mode と RIo の 4 欄）
  - [x] IO#puts / #print / #<< / #printf と Kernel#puts / #print / #p / #putc が上書きされた #write を呼ぶ（rb_io_write / rb_io_writev、アリティ 1 の #write には 1 個ずつ。write / puts / putc の定義を redef_note が見るまではフラグ 1 個の検査だけ。io/puts 5 → 18 / 19、~/ruby/src/trials/2026-11-23-saruby-compat-bundle-2/）
  - [ ] 残り: io/puts の 1 件は rb_check_funcall（to_ary を method_missing 経由で呼ぶ）が check_array に無いため。write を定義した module が後から IO に入っていない場合も遅い道（lookup）に入る
  - [x] UTF-16 / UTF-32 の String メソッド（chomp / chop / strip 系 / lines / 揃え / partition / 大小文字を UTF-8 形で）、Integer#chr(enc)、sprintf の非 ASCII 互換引数（~/ruby/src/trials/2026-11-23-saruby-compat-bundle/、差分調査 407 → 33 行）
  - [x] Symbol のエンコーディング（UTF-16/32、非 ASCII の BINARY / ISO-8859-x / EUC-JP / SJIS / 高位バイトの US-ASCII は別 Symbol: symbol.rs の副表、intern は不変）、Symbol#encoding / to_s / inspect / length、非 UTF-8 ソースの識別子・リテラル・ラベル、eval の文字列のエンコーディング、Marshal、sprintf の %<name>、const_get。壊れた文字列の strip / lstrip / rstrip の ArgumentError / CompatibilityError（全エンコーディング、~/ruby/src/trials/2026-11-23-saruby-compat-bundle-2/）
  - [ ] 残り: 壊れた UTF-8 の String#to_sym が EncodingError を出さない（検査は to_sym 1 回 +6 命令、見送り）、interpolation `:"#{...}"` の壊れたバイト（node_dsym）、Struct / TracePoint 等が文字列名を intern する所はエンコーディングを見ない、UTF-32 で 0x80000000 以上の符号（Onigmo は符号付きで比較して有効扱い）の valid_encoding?
- [x] blop 層（docs/io_design.md、~/ruby/src/trials/2026-11-22-saruby-blop/）: 待ちは全部 slab 所有の op、エンジンは poll（既定）/ epoll。1 ms ポーリング廃止。file/flock・file/open・io-wait/wait・process/status/wait のタイムアウト解消
- [ ] まだタイムアウト: kernel/require（並行 require: ロード中 feature の待ちと他スレッドの Thread#backtrace が要る）
- [ ] blop: epoll の登録を fd 単位で持ち越す、io_uring エンジン、CFUNC ワーカー（flock / FIFO open / waitpid(-1) のバックオフ代替、getaddrinfo）
- [ ] Queue ping-pong 1.7M → 1.0M round trips/s（blop の簿記）
- [x] `$~` / `$_` をメソッドのフレームローカルに（ブロックと共有、builtins/svar.rs: 呼び出しごとのコスト 0、`$~` を書く照合ごと +101〜151 命令）、`$?` はスレッドローカル、`$0 =` は /proc/self/comm（~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [ ] svar の残り: 生きているメソッドのフレームのブロックを別ファイバー / スレッドで呼ぶと `$~` を共有しない（id だけで引く）、死んだ Binding への eval の return（base と同じくプロセスが黙って終わる）、`b.binding.local_variable_set` した変数を eval から見えない、`$0 =` は CRuby と違い argv 領域（ps の COMMAND）でなく comm を変える
- [x] library/ 2496 → 6156 / 6376（CRuby 6401 / 6402、~/ruby/src/trials/2026-11-22-saruby-stdlib-2/）: digest、zlib（zlib 1.3 の deflate / inflate を移植、出力はバイト一致）、psych / yaml（libyaml 0.2.5 の移植）、bigdecimal（bigdecimal.c の移植）、socket（+ net-http / net-ftp / ipaddr / resolv）、StringIO（stringio.c 3.2.0 の移植に置き換え）、syslog、etc の sysconf / confstr、cgi/escape.so、coverage（lines / oneshot / methods / branches / eval）、rbconfig/sizeof
- [x] objectspace / weakref（~/ruby/src/trials/2026-11-23-saruby-objspace/）: objspace.so（builtins/objspace.rs、forbid）と prelude/lib/objspace.rb・objspace/trace。trace_object_allocations は GC の割り当てフック（`Gc::set_alloc_hook`、stress と同じ 1 回の検査に相乗り: 止めている間の割り当ては命令数 0 増）、dump / dump_all / memsize_of / reachable_objects_from / count_* / internal_*。弱参照を GC に（`GcLang::sweep_weak` + `WeakVisitor`、両 backend）: WeakMap / WeakKeyMap は隠し Hash（FL_WEAK_KEYS / FL_WEAK_VALS）、WeakRef の参照先が GC.start で回収される。object_id の表を解放時に消す（FL_HAS_ID: 番地を再利用した別オブジェクトが古い id を持っていた）、_id2ref、each_object の Enumerator、define_finalizer の重複・警告、-W0。rubyspec library/objectspace 0 → 46 / 46、library/weakref 7 → 9 / 9、core/objectspace 93 → 105 / 107（CRuby 4.0.2 は 3 dir とも全件）
- [ ] objectspace の残り: core/objectspace/each_object の 2 件（インスタンスの特異クラスのクラスがクラスの特異クラスでない: 特異クラス階層の問題）、弱参照の回収は保守的スタック走査の残りがある分だけ不完全（仕様上「大半が消える」までしか観測できない）、WeakKeyMap の値は強参照（CRuby も ephemeron ではない）、dump の slot_size / memsize は saruby の値（56 バイト slot + 所有するバッファ）、internal_super_of に iclass が無い
- [x] ファイナライザを対象が死んだときに走らせる（~/ruby/src/trials/2026-11-23-saruby-finalizer-scheduler/、docs/gc.md）: 表は弱キーの隠し Hash（object → [id, callable...]）、sweep_weak が死んだ対象の配列をキューへ移して割り込みビットを立て、次の検査点（thread::poll）か GC.start の終わりで実行。CRuby と同じく 1 オブジェクト内は定義順・例外は警告して次へ・実行中の GC は追記だけ・終了時は登録順 → キュー・dup / clone で複製。終了時の順序が逆だった（base は後から定義したものが先）のも直した
- [ ] ファイナライザの残り: 例外の backtrace に prelude の GC.start / GC.__start の枠が出る（CRuby は `from x.rb:N:in 'GC.start'` だけ。prelude の枠一般の問題）、ファイナライザ実行中に Thread#raise を遅らせない（CRuby は PENDING_INTERRUPT_MASK）
- [ ] `Coverage.line_stub`: CRuby は `RubyVM::InstructionSequence.compile_file(file)` の `trace_points` で行を拾う（coverage.rb）。saruby に InstructionSequence は無い。代わりにコンパイラの coverage 行スロット（compile_cov.rs）を実行せずに取り出す経路を作れば作れるが新しい機構なので未実装
- [ ] library/ の残り: openssl 173（部分実装は `defined?(OpenSSL::SSL)` 判定を壊すので丸ごと）
- [ ] 起動の命令数が codegen-units=16 の分割で ±0.8M 動く（`__rust_alloc` がどの呼び出し元に inline されるか）。比較は `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` で（base 16.177M → stdlib-2 16.190M。既定ビルドの先頭は json の FxHash 化で 15.91M）
- [ ] 値を使わない式（`if` の値など、外側から決まるもの）の line イベントがまだ CRuby より多い（文単位の省略はした）
- [x] autoload が `main.require` を呼ぶ、Kernel#autoload の対象は rb_vm_cbase 相当、to_path / 凍結 / 定数名の検査、Enumerator#first / take を別スレッドから供給したときの break（LocalJumpError をそのスレッドで、first_i / take_i の数え方）。kernel/autoload 14 → 24 / 25（~/ruby/src/trials/2026-11-23-saruby-compat-bundle-2/）
- [ ] autoload の残り: Class.new のブロック中の def の cref（kernel/autoload 1 件）、module/autoload 17 件（ロード中の他スレッドからの見え方、同じファイルを直接 require したとき、const_source_location、rubygems の読み込み）
- [ ] RUBY_VERSION を 4.0.x にするか（いまは 3.4.0。mspec は SPEC_RUBY_VERSION で回避）。変わるもの（調査のみ、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/ README）: RbConfig の MAJOR/MINOR/ruby_version/RUBY_API_VERSION も揃える必要（Gem.default_dir が lib/ruby/gems/3.4.0 を指し 4.0.2 の gem が見えない）、4.0.2 の rbconfig.rb は 4.0.x 以外で raise、bundled_gems.rb の SINCE 判定（ostruct / logger / benchmark / pstore / irb 等が 3.4.0 では :warning、4.0 では :error 側）、gem の `require "cgi/util" if RUBY_VERSION < "3.5"`（3.4.0 だと 4.0 で消えた CGI::Util を読みに行く）、prism ffi の version 3、Bundler の ruby_40?、RUBY_DESCRIPTION の "Ruby 3.4 compatible"
- [x] CRuby benchmark/*.rb 148/148（yml は 228/236: IO::Buffer、RubyVM、irb、Pathname、fiber_chain）
- [ ] ruby-bench 69/88（check、plain。stdlib-2 で zlib・psych・json/ext が入り chunky-png・hexapdf・psych-load・liquid-*・json_parse_* が一致、Ractor で knucleotide-ractor・ractor-idle-garbage が一致）。残り: ractor-conservative-stack（mkmf で CRuby の C 拡張をビルドして require する: saruby では不可）、未インストール gem 18 本
- [x] Ractor（docs/ractor.md、~/ruby/src/trials/2026-11-23-saruby-ractor/）: CRuby 4.0 の API（new / join / value / Port / select / monitor / receive / main / count / Ractor-local / make_shareable / shareable? / shareable_proc / RemoteError / ClosedError / IsolationError / MovedError / MovedObject）を緑スレッドの上に。メッセージと引数は深いコピー、move: で送り手のオブジェクトは MovedObject に。非 main Ractor からの非共有定数・グローバル・クラス ivar・クラス変数・fork は IsolationError、ブロックの外側変数と yield は ArgumentError。検査は遅い経路だけで、他の Ractor が生きている間だけ（定数キャッシュはその間は共有可能な値だけ保持）。15 ベンチ interp / AOT は +0.1 % 以内（多くは同じか減少）、起動 +0.12〜0.16 %、optcarrot 83.15M/frame で同じ。t/hand の既存 Ractor テスト 4 本と新規 7 本が一致
- [ ] Ractor の残り: 並列実行なし、`@x` によるクラス ivar の読みの検査は次のチェックポイント（node_ivar の miss 経路が RV を返すと poly が 1.4〜2 % 増えるため）、外側変数の検査は Ractor の名を含むソースのブロックだけ（`Ractor.new(&pr)` の pr が別ファイル）、$stdout 等は Ractor ごとでない、Mutex のコピー、ractor の終了時に他スレッドを殺さない、moved オブジェクトの object_id
- [x] `Date` / `DateTime`（prelude/lib/date.rb、ext/date の Ruby 移植）、`Pathname`（CRuby 4.0 同様に組み込み、遅延ロード）、`require 'time'` は CRuby の time.rb
- [x] Fiber storage、`Fiber#blocking?`、fiber-local `Thread#[]`、`Fiber#inspect` の状態、fiber called across threads
- [x] Fiber scheduler（docs/io_design.md「Fiber scheduler」、~/ruby/src/trials/2026-11-23-saruby-finalizer-scheduler/）: Fiber.set_scheduler / scheduler / current_scheduler / schedule、フック kernel_sleep・block / unblock（Mutex・Queue・SizedQueue・ConditionVariable・Thread#join、blop の待ち行列に Wake op を置いたまま呼ぶので通知の順序はスレッドと同じ）・io_wait / io_read / io_write・io_select・io_close・fiber_interrupt・process_wait・address_resolve・timeout_after・fiber・close（差し替え・スレッド終了・終了時）。Mutex の所有はファイバー単位（CRuby の ec）、fork した子はスケジューラ無しで blocking。`Fiber#raise` / `Thread#raise` は rb_exception_setup（cause: は呼び出し側の `$!`、only cause、circular causes、余分なキーワード）、`Fiber#kill` の 2 回目は false、`Fiber#backtrace(_locations)`（現在のファイバーのみ）、`Process::Status.wait`、Kernel#sleep / Mutex#sleep の rb_time_interval（divmod）。CRuby の test/fiber/*.rb（shim で両方に流す）は 14 ファイル 102 / 104 件一致（残りは autoload の並行読み込みと RubyVM）
- [ ] Fiber scheduler の残り: blocking_operation_wait、io_pread / io_pwrite、yield（非ブロッキングファイバーの Thread.pass）、IO::Buffer#read / #write からの io_read / io_write、stdout / stderr への書き込みは io_write を通らない（プロセスのバッファ）、他のファイバーの Fiber#backtrace は []、ロード中の autoload を別ファイバーが待たない、unblock は通知の中でなく通知した側の次の検査点 / 待ちで呼ぶ（通知の経路に検査を足さないため）、Thread#join の unblock は終わるスレッドでなくメインスレッドで走る
- [x] Enumerator がキーワード引数を渡さない・private メソッドを呼べない（rb_block_call と同じに、stdlib-2）
- [x] 再定義フラグを 64bit に（REDEF / Ctx::redef、~/ruby/src/trials/2026-11-23-saruby-redef-flags/ README に CRuby 4.0 の BOP 表と saruby の近道の対照）。case/when・case/in・splat when が再定義・refine された Integer / Symbol / nil / true / false / String#=== を呼ぶ（即値は R_EQQ_IMM 1 回の検査、CRuby の opt_case_dispatch と同じ）。近道の棚卸しで: Integer#!= / Float#!=（== のフラグに相乗り: R_INT_ALL を bit 0..30 に収めて imm32 の test 1 つのまま。bit 38 に置いた版は movabs が増え +0.6〜1.8 % 命令で撤回）、String#==、条件形の Symbol#==、即値の `!`、refine された `!`、式展開の Integer / Symbol / nil#to_s、attr_* / private 等の可視性変更 / import_methods が再定義を通知しなかった。15 ベンチ interp / AOT は ±0.01 % 以内（mapreduce AOT −1.0 %）、起動同じ、optcarrot +0.11 %/frame。t/hand で既存 5 本が CRuby と一致に
  - [x] 組み込みメソッド側（~/ruby/src/trials/2026-11-23-saruby-redef-builtins/）: R_INT_CMP / R_FLO_CMP / R_STR_CMP（OPTIMIZED_CMP: sort / max / min / sort_by / Range#include? / ===）、Array#max(n) / min(n) / min_by(n) / max_by(n) は rb_nmin_run の移植、Array#minmax は [min, max]（CRuby のビルドでは max が先）、Enumerable#min / max / minmax / min_by / max_by / minmax_by と Comparable#< <= > >= between? clamp を native に（enum.c / compar.c と同じ比較順）、`[a, b].max` / `.min`（非リテラル要素）は node_newarray_minmax（opt_newarray_send）、R_IMM_INSPECT（p / Array#inspect / %p）、R_HASH_DEFAULT（rb_hash_default_value）、R_STR_FREEZE / R_STR_UMINUS（`"lit".freeze` に新しい複製: node_fstr_recv）、when / in の `===` が例外を出すとパターンの行（node_when に行オペランド、行ごとに分割）
  - [x] 差分調査（Integer / Float / String / Symbol / Array / Hash / Range / nil の 30 メソッド × 約 240 の組み込み呼び出し、trial の bin/survey.rb）で見つかった安いもの: rb_equal の近道が == のフラグを見る・Symbol 引数で obj#== を呼ぶ、Array 自身で再定義した Array#hash、inject(:+) の Integer#+、Range#sum の整数公式は Integer#+ を呼ばない、&:sym の Symbol#to_proc、Array / Range の Enumerable メソッドが再定義された #each を呼ぶ（R_ARY_EACH / R_RANGE_EACH、焼いた inject / each_with_index も）、即値に定義した to_str / to_ary を変換が見る（R_IMM_CONV: join / flatten / assoc / puts / Array() / String() / String#% / 多重代入 / ブロックの自動展開）、uniq は要素ごとに hash 1 回、join の先読みの to_str。不一致 220 → 93。費用（命令数、CU=1）: 15 ベンチ interp / AOT とも −0.005〜−0.37 %（nesteddata −1.2 / −1.7 %）、起動 14.10M → 13.47M（prelude の縮小）、optcarrot 83.18M → 83.15M/frame。マイクロ: sort ±0.3 %、max/min −70 %、sort_by/min_by −60 %、Comparable −47 %、include? −26 %、`[x, y].max` −23 %、Hash#[] の未登録キー +1.0 %、join +2.2 %（CRuby と同じ先読み）、`"lit".freeze` +2.3 / +2.8 %、Array#inspect +0.7 %
  - [x] pp / PP / PrettyPrint は CRuby 4.0.2 が有効にする pp 0.6.4 / prettyprint 0.2.1 gem の lib をそのまま vendoring（prelude/lib/pp.rb・prettyprint.rb、初回 require でコンパイル）。Kernel#pp は CRuby の prelude.rb と同じ定義（require 'pp' して呼び直す）なので最初の pp まで PP は未定義。native の builtins/pp.rs は削除。差分調査の残りも削減（~/ruby/src/trials/2026-11-23-saruby-pp-prelude/）: partition / tally を native に、prelude の `|*x| x.size <= 1` の詰め方を Enumerable.__each に、Set の initialize / merge / == を native 補助に、Integer#times / digits / step・Range#step・ArithmeticSequence#each が再定義された < / succ / == / >= / coerce を CRuby と同じに呼ぶ（R_INT_SUCC / R_INT_COERCE）、小さい配列（16 以下）の - & | difference union intersection intersect? は rb_eql の線形探索、Integer#eql? が再定義された == を呼ぶ、each_slice / each_cons が size を聞く（R_ARY_SIZE）、Complex の f_add / f_sub / f_mul、再帰検査の Hash（Thread.current[:__recursive_key__]、pp.rb と :inspect のリストを共有、Hash#default 再定義時の rb_hash_aref）。不一致 93 → 28（--disable-gems の CRuby 相手では 10）。命令数（CU=1、sp4 と手元）: 15 ベンチ interp −0.067〜+0.007 %、AOT −0.14〜−0.014 %、起動 −3〜−4 %、optcarrot 83.12M/frame で同じ。マイクロ: tally −89 %、set −71 %、partition −70 %、Array の集合演算 −30 %、Enumerable（grep / first / take_while / all? / sum）−25 %
  - [ ] 差分調査の残り 28 件: 20 件は CRuby の rubygems が pp gem を有効化するときの Ruby コード（Gem::Specification 等が Integer#== / String#== / Array#+ / Array#hash / Symbol#to_proc / Hash#default 等を呼ぶ）、1 件は CRuby のコンパイラが負の整数リテラルに Integer#-@ を呼ぶ（prism_compile.c parse_integer_value、prettyprint.rb の読み込み時）、7 件は Lazy（prelude の Ruby 実装。CRuby は Range#each が 1 <=> Infinity・succ・to_str を呼ぶ）。ArithmeticSequence#first と Integer#step のブロック形は CRuby の呼び出しを未確認。Module#class_variables(false) が継承分を返し、順序が Sym の HashMap 順
  - [ ] AOT で落ちる（base 70fcb06 でも、`require "pp"` で CRuby の pp.rb を読んでも）: `pp 1; pp Struct.new(:q).new(5); class Foo ...; Foo.new.pretty_inspect` を焼いて実行すると call_slow（vm.rs）が壊れた CallCache を読んで SIGSEGV（~/ruby/src/trials/2026-11-23-saruby-pp-prelude/aot_crash_repro.rb）。--plain は正常
  - [x] ブロックの捕捉で Proc に昇格（CRuby の rb_vm_make_proc と同じ規則、~/ruby/src/trials/2026-11-23-saruby-block-escape/）: enum_native.rs の native Enumerable（map / select / find / group_by / inject / each_slice / min_by(n) の Nmin ...）が Ruby ブロックのアドレスを cfunc ブロックのデータに整数で入れていた（#each がブロックを保存して後で呼ぶと解放済みスタックを読む、base で SIGSEGV）。cfunc ブロックは outer_blk で呼び出し元のブロックを持ち（CBlockFn の第 5 引数、Block::cfunc_for）、捕捉時に block_to_proc が outer ごとヒープへ写す。Binding は枠のブロックを Proc で持つ（eval の yield / block_given?）。組み込み側の CFunc / CBlockFn は Blk<'call>（vm.rs）で、保存は E0106 / E0521、整数化は E0605、生ポインタは heap_audit.rb が拒否。後呼びの find / include? / Enumerator#first は LocalJumpError（unexpected break）。unsafe_catalog.md 2.4.3・行 45〜49
  - [ ] 後から呼ばれた保存ブロックの戻り値・例外が CRuby と違うもの（prelude の Ruby 実装: first / sort_by / lazy / sum / each_entry / zip / sort / uniq / chunk_while、cfunc Proc の arity が -1（CRuby の ifunc は 1））。メモリ安全性の問題ではない（trial の t/kept_prelude_1123b.rb）
  - [ ] `eval("class K2; eval('yield'); end")` が CRuby ではコンパイル時の SyntaxError（クラス本体の yield）、saruby は実行時の LocalJumpError（t/hand gem_w19_eval_yield_1027.rb の最後、compile.rs 側）
  - [ ] block_to_proc が呼び出し元の Block に *const 経由で proc_val を書く（共有参照由来のポインタへの書き込み）。Block が Copy なので Cell にできない
  - [ ] Kernel#inspect が CRuby 4.0 の instance_variables_to_inspect を見ない（pp.rb の pretty_print_instance_variables は見る）
- [x] 再帰的な正規表現サブ式呼び出し `(?<a>a\g<a>?)`・`\g<0>`・レベル付き後方参照 `\k<a+0>`・absent 演算子 `(?~x)`（2026-11-23-saruby-regexp）
- [x] blop 層（docs/io_design.md）: 待ちを 1 か所に、slab + Id + 所有権ごと渡すバッファ、engine trait（poll 既定・epoll）。待機中の CPU ほぼ 0、外部 cat との往復 1k → 45k/s。rubyspec file/flock・file/open・io-wait/wait・process/status/wait のタイムアウト解消
- [ ] blop の残り: Mutex/CV/Queue の受け渡しが重い（vm_thread_condvar2 +59%、condvar1 +28%、sized_queue4 +26% 命令）、epoll の登録キャッシュ、io_uring engine、worker thread（flock・FIFO open・Process.wait(-1) は back-off の仮実装）
- [x] 時分割（docs/io_design.md「Time slice」、~/ruby/src/trials/2026-11-23-saruby-timeslice/）: 割り込みフラグはメソッドシリアルの最上位ビット（呼び出しサイトのキャッシュ比較がすでに読む語: ヒット側の命令 0、ミス側 call_slow で poll）、相手を待ちうる while / until には compile.rs が node_poll を被せる（数えるループと条件が必ず呼ぶループには付けない）、Kernel#loop は 1 周ごと。タイマーは 10 ms ごとの std::thread（他スレッドがいて pump で寝ていない間だけ、fork 後は作り直し）、量子 100 ms を Thread#priority で伸縮。Thread#raise / #kill はキュー、Thread.handle_interrupt（:immediate / :on_blocking / :never）、pending_interrupt?、trap / Timeout が忙しいループを割り込む。rubyspec mutex/lock・thread/list のタイムアウト解消、handle_interrupt 1 → 7/7、pending_interrupt 2 → 3/3、priority 4 → 7/7、:on_blocking は puts / print / IO#write でも配る。icount の 15 ベンチは interp / AOT とも +0.015 % 以内（poly は −0.4 / −0.9 %）、起動 +0.18〜0.27 %（AST の drop glue のインライン化が動いた分）、optcarrot +0.08 %/frame、vm_thread_* は close +19 %（1000 スレッドが動くようになり IO#close が cancel_fd を通る）・create_join +1.6 %・pass_flood +1.8 % 以外は −0.7〜−2.1 %、puts_loop +0.37 %（呼び出しごとに待ち割り込み数の load 1 回）。全後方枝に検査を置いた版は +1〜9 % で却下（機械語は trial の results/）
- [ ] 時分割の残り: 呼び出しの無いブロックを回す組み込みイテレータ（`10**8.times { x += 1 }`）と、本体が代入するローカルだけを条件に読むループは終わるまで横取りされない（CRuby は後方枝ごとに検査）。他スレッドの Thread#backtrace が空
- [ ] `Array#shift` が O(n)（so_lists）。O(1) 版 2 案は他の配列アクセスを遅くして却下済み
- [ ] 大きな左シフト（CRuby は NoMemoryError、saruby は 2^26 bit 超を RangeError）
- [x] `__callee__` on alias、`WeakMap#inspect`、`detailed_message` / `full_message` の書式（eval_error.c / error.c の移植、~/ruby/src/trials/2026-11-23-saruby-compat-bundle/）
- [x] error_highlight（~/ruby/src/trials/2026-11-23-saruby-error-highlight/）: CRuby の lib/error_highlight（ruby_4_0 の 0.7.2）をそのまま同梱（base.rb の変更 3 か所、RubyVM / Prism は ErrorHighlight の下に saruby.rb で用意）。ノードは生成元の prism ノード id を head の flags の上位ビット（NF_TAG_SHIFT、astrustgen が埋め込み側に開放。ハッシュ外・layout_tree で複製・メモリ 0）に持ち、フレームの Pos がそのノードを指す（行でないものは Pos::info で bit 1、patch 束縛の imm32 は BOUND 表の添字）。報告時にファイルを prism で読み直し id の節を引く（prism_generic.rs、SCHEMA）。CoreExt は起動時に prepend（スタブ）、本体は初回に読み込み。CRuby の test/error_highlight 120 件中 119 一致（残りは RubyVM 専用）。比較: operand 案は optcarrot で +40KB、side table 案は起動 +3.5 %。起動 +0.72 %（sp4、prelude は札付けしない）、15 ベンチ +0.03 % 以内（mapreduce AOT +0.14 %）、optcarrot 同じ。raise は −0.4 %（行と id は整形時に読む）、caller_locations −51 %
- [ ] error_highlight の残り: did_you_mean が無い、起動 +0.72 %（予算 0.3 %: コンパイラのコード生成の揺れ ~70K と node_allocated の判定）、`[1].first(1, 2)` が ArgumentError にならない（Array#first の引数検査）
- [ ] t/hand: 約 23,400 PASS / 10,200 FAIL
- [ ] 解説書『saruby でわかる Rust』（~/ruby/src/trials/2026-11-21-saruby-rust-book）: vm/heap 分割の反映、trait と Ruby の module の対比・dispatch（Dyn と T_… の Disp）の説明を追加（2026-11-22 の会話）
- [x] `Array#cycle(n).size` が i64 で `len * n` してあふれる: Bignum で返す（CRuby master と同じ。4.0.2 は LONG2FIX で折り返す）。同時に Enumerable.__int の Fixnum 折り返しと Enumerator#size が Bignum を nil にするのも修正
- [x] `cycle` の size を遅延計算: `[1].cycle(2**64)` は Enumerator、size で RangeError（rb_ary_cycle_size、~/ruby/src/trials/2026-11-23-saruby-compat-bundle-2/）
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
