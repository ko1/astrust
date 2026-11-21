# saruby TODO

優先順位（2026-11-21 user 指示）: **1. ASTro 原則 → 2. unsafe → 3. CRuby 互換性**。性能はこの 3 つの下。
新しい作業を起こす前にこの順に照らす。計測は `~/ruby/src/trials/`、採否の記録は `~/ruby/src/ideas/`。

## 1. ASTro 原則

原則の出典: `~/ruby/astro/docs/perf.md` §0、`docs/idea.md`、`docs/idea_code_store.md`。
監査: `~/ruby/src/trials/2026-11-21-saruby-astro-principles/README.md`（Audit 1, 2）。

- [x] 行番号を構造ハッシュから外す（`@hole`、フレームは行への参照）。iseq / CallInfo / ヒープリテラルを内容でハッシュ
- [x] patch 束縛（初回ディスパッチで穴を即値に。koruby の既定に合わせ既定 ON、`SARUBY_PATCH=0` で無効）
- [x] 再利用テスト `tools/reuse_test.sh`（別プログラムで焼いたストアが効くこと）
- [ ] **焼き込むプロファイル値をハッシュに入るノードパラメータにする**（作業中）。SD 名が焼いた内容をすべて反映するように
  - ivar スロット番号（user: 「ノードのパラメータにして焼くだけなら原則を破らない」）
  - site kind、`shared`、組み込んだ呼び出し先・ブロック、演算子の型、イテレータの選択
- [ ] 組み込みメソッドの展開を名前で選ぶ（F4、採用決定・マージ待ち）
- [ ] inline iterator / inline builtin を builtin 自身のループ・関数を呼ぶ形に（F2, F3、マージ待ち）
- [ ] EVAL body の残りの手書き fast path を特殊化か parser-pass へ
  - `node_block` の手展開、`node_ary` の小配列経路 → 生成器でリストを展開
  - 呼び出しの最初の 3 引数の手展開
  - `send` + リテラル Symbol → compile.rs で通常の呼び出しノードに書き換え（Kernel#send ガード付き）
  - `<<` `[]` `[]=` の Array/String/Hash 経路 → builtin 自身の関数を呼ぶ
- [ ] `yield_block` 系ヘルパを 1 コピーに固定（インタプリタの命令数が codegen unit の分割で動かないように）

## 2. unsafe

「メソッド実装で使っていなければだいぶいい」（user）。src/builtins/ は sys.rs（OS インターフェース）以外 `forbid(unsafe_code)` 済み。
crate 全体 364（2026-11-21 d3400bc）。分類と経緯は `docs/safe_heap.md`。

- [x] builtins（全波）、inline_builtin.rs / inline_iter.rs、string/array/hash/range/bignum/regexp 本体、binding.rs、boot.rs
- [ ] sbuf.rs（String のバッファ、10 か所）: safe 版はブランチ `sbuf-safe-rs29`（str AOT +6% 命令 / +11.5% サイクル）。採否未定
- [ ] 呼び出し経路・フレーム連鎖（vm.rs 80、rt.rs 64、node.def 16、stack_raw.rs 5）。前提 H5（実行中の builtin の下のフレームは生きている）は未検証
- 残す（外部インターフェース・土台）: sys.rs、kalloc.rs、fiber_os.rs、patch ローダ、heap.rs / value.rs / gc.rs の土台

## 3. CRuby 互換性

- [ ] **rubyspec を実 mspec で計測し、失敗の多い所から直す**（作業中）
- [x] CRuby benchmark/*.rb 148/148（yml は 228/236: IO::Buffer、RubyVM、irb、Pathname、fiber_chain）
- [ ] ruby-bench 60/88。残り: Zlib、Psych(YAML)、Date、json の C パーサ、Ractor、未インストール gem 18 本
- [ ] `Date`（`require 'date'` が何もしない）、`Pathname`
- [ ] Fiber storage、`Fiber#blocking?`、Fiber scheduler、fiber-local `Thread#[]`
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

## 計測の約束

- 平均（geomean 等）は出さない。ベンチごとの実測値とばらつき
- 起動時間は `_startup` 行で分離。0.1 秒級のベンチは起動で律速するので長いベンチを使う
- sp4 は bench-lease。ローカル（Zen 3、共有）は命令数のみ信頼
