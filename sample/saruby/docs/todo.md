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
  - [ ] 起動 +2M 命令（索引の読み込み等、未分解）、bake 時間への影響未測定
  - ivar スロット番号（user: 「ノードのパラメータにして焼くだけなら原則を破らない」）
  - site kind、`shared`、組み込んだ呼び出し先・ブロック、演算子の型、イテレータの選択
- [x] 組み込みメソッドの展開を名前で選ぶ（F4）。size/length/empty?/to_i/to_int は +6〜20 命令/呼び出し
- [x] inline iterator / inline builtin を builtin 自身のループ・関数を呼ぶ形に（F2, F3。iterators −11% サイクル）
- [x] EVAL body の残りの手書き fast path を特殊化か parser-pass へ（Round 3）
  - [x] `node_block` / `node_ary` / 呼び出し引数の手展開 → 生成器の `EVAL_EACH!`（特化でリストを展開、AOT 命令数は base と同じ）
  - [x] `send` + リテラル Symbol → node_send_sym（compile.rs、Kernel#send ガード付き。u_send_lit 381 → 146 命令）
  - [x] `<<` `[]` `[]=` → builtin 自身の関数。凍結時のバックトレースが CRuby と一致。String#<< は safe な追記経路で str +10% 命令、Hash#[]= は行の保存で hash +2%、Array#<< は ary +3%（判断待ち）
- [x] `yield_block` を 1 コピーに（interp hashiter +7.9% / mapreduce +2.1%、AOT は同じ）。lookup_method / new_object / invoke / raise_exc は 1 コピーにすると nesteddata +9% なので残す

## 2. unsafe

「メソッド実装で使っていなければだいぶいい」「unsafe 外しは性能と見合い」（user）。外す前にコストを測り、遅くなる所は数字を添えて残す。src/builtins/ は sys.rs（OS インターフェース）以外 `forbid(unsafe_code)` 済み。
crate 全体 364（2026-11-21 d3400bc）。分類と経緯は `docs/safe_heap.md`。

- [x] builtins（全波）、inline_builtin.rs / inline_iter.rs、string/array/hash/range/bignum/regexp 本体、binding.rs、boot.rs
- [ ] **コスト 0 で外せる 25 か所を入れる**（目録で測定済み、patch は ~/ruby/src/trials/2026-11-22-saruby-unsafe-catalog/patches/）: Value 構築とリテラルキャッシュを NonZeroU64 / Option<Value> に（−13）、REFINEMENTS_ON・BOOTED を AtomicBool に（−10）、CallCache.cfunc を Option<fn> に（−2）。原則 3 回目のマージ後
- [ ] unsafe_catalog.md 表 2.4 の 39 件に「どうやって見つかったか」列（型検査 / 借用規則 / 読み直し・テスト）。論文で safe 化の効果を主張する根拠（2026-11-22 user「safe 化のいい話」）
- [ ] 安全なシグネチャで `&'static mut` を返す関数 10 個と、その呼び出し 449 か所（unsafe の数に出ない信頼の穴。docs/unsafe_catalog.md）
- [ ] sbuf.rs（String のバッファ、10 か所）: safe 版はブランチ `sbuf-safe-rs29`（str AOT +6% 命令 / +11.5% サイクル）。採否未定
- [ ] 呼び出し経路・フレーム連鎖（vm.rs 80、rt.rs 64、node.def 16、stack_raw.rs 5）。前提 H5（実行中の builtin の下のフレームは生きている）は未検証
- 残す（外部インターフェース・土台）: sys.rs、kalloc.rs、fiber_os.rs、patch ローダ、heap.rs / value.rs / gc.rs の土台

## 3. CRuby 互換性

- [x] rubyspec を実 mspec で計測（runner: ~/ruby/src/trials/2026-11-22-saruby-rubyspec/bin/、SPEC_RUBY_VERSION=4.0.2）。core 19542/22373、language 2592/2933、library 1922/3188（CRuby 4.0.2: 23144/23155、2934/2934、6401/6402）
- [ ] **AOT で knucleotide が segfault**（ruby-bench、base から。最優先で調べる）
- [ ] rubyspec の大きな塊: IO::Buffer 180、Encoding::Converter 167、TracePoint 72、#inspect の違い 101、引数の型チェック 90、Array#== / #hash の端 90、NameError#receiver 60、language の非 UTF-8 マジックコメント 30・predefined_spec 79、core のタイムアウト 14 ファイル
- [ ] library/: bigdecimal、socket（core/file の 124 も）、zlib、digest、net-*、yaml
- [ ] RUBY_VERSION を 4.0.x にするか（いまは 3.4.0。mspec は SPEC_RUBY_VERSION で回避）
- [x] CRuby benchmark/*.rb 148/148（yml は 228/236: IO::Buffer、RubyVM、irb、Pathname、fiber_chain）
- [ ] ruby-bench 60/88。残り: Zlib、Psych(YAML)、json の C パーサ、Ractor、未インストール gem 18 本（liquid-* は Date の次に YAML で止まる）
- [x] `Date` / `DateTime`（prelude/lib/date.rb、ext/date の Ruby 移植）、`Pathname`（CRuby 4.0 同様に組み込み、遅延ロード）、`require 'time'` は CRuby の time.rb
- [x] Fiber storage、`Fiber#blocking?`、fiber-local `Thread#[]`、`Fiber#inspect` の状態、fiber called across threads
- [ ] Fiber scheduler、`Fiber#raise(cause:)`
- [ ] Enumerator がキーワード引数を渡さない（`to_enum(:m, k: 1)`）・private メソッドを呼べない（`Pathname#find` の enumerator が落ちる）
- [ ] case/when が再定義された `Integer#===` を無視する（再定義フラグ 32bit が満杯 → 64bit に）
- [ ] 再帰的な正規表現サブ式呼び出し `(?<a>a\g<a>?)`
- [ ] `Array#shift` が O(n)（so_lists）。O(1) 版 2 案は他の配列アクセスを遅くして却下済み
- [ ] 大きな左シフト（CRuby は NoMemoryError、saruby は 2^26 bit 超を RangeError）
- [ ] `__callee__` on alias、`WeakMap#inspect`、`detailed_message` / `full_message` の書式
- [ ] t/hand: 約 23,400 PASS / 10,200 FAIL

## 性能（上 3 つの後。新規には起こさない）

- [x] sp4 の退行 ary: glibc がメモリをカーネルに返して再フォルト → `M_TOP_PAD` 64 MiB（ary 110 → 68 ms）
- [x] sp4 の退行 ackermann: patch 束縛ではなく all.so と patch.o で引数リストの名前付けが違い別の木を比べていた → 名前付けを揃えた（a1246fd）
- [ ] method_call/send の 3dd858b 退行（Zen 4）: 1 呼び出しあたりストアが 1 つ多い、出所未特定（Zen 3 の memory disambiguation とは別）
- [ ] bake を SD 単位にコンパイル・キャッシュ（いまはプログラムごとに all.rs 4.6 MB + patch.rs 7.9 MB を 1 crate で。optcarrot 47 秒、30k_* は rustc がメモリ不足）
- [x] 長いベンチ（`bench_long/`、本体約 1 秒）での比較を artifact に（long.html）
- [ ] koruby との差（起動差し引き、sp4）: tak・gcd・fib・ackermann（呼び出し 1 回の命令数）、bitops・intdiv（slow path 合流後の再検査）、optcarrot の L1d
- [ ] 受け手をレジスタに保持（method_call −7% / optcarrot +0.4% 命令、判断待ち）
- [ ] 起動: prelude の def 本体の遅延コンパイル（残り 13.7M 命令のうち 6.8M）
- [ ] 起動の後戻り: `-e 'p 1'` 13.7M（de58efd）→ 20.6M 命令（2026-11-22、hopt_index・Pathname の起動時定義ほか）。`require 'date'` は +13 ms/プロセス
- [ ] ivar の多相: 親クラスのメソッド内の `@a` が親・サブクラス両方のオブジェクトから使われると並びが違い、単相のキャッシュが外れる（shape 的な仕組み）。スロット番号は実行しないと決まらない（継承・条件分岐・instance_variable_set）ので静的割り当てはしない（2026-11-22 user と確認）

## 計測の約束

- 平均（geomean 等）は出さない。ベンチごとの実測値とばらつき
- 起動時間は `_startup` 行で分離。0.1 秒級のベンチは起動で律速するので長いベンチを使う
- sp4 は bench-lease。ローカル（Zen 3、共有）は命令数のみ信頼
