# Rust で Ruby を実装して分かった「Rust ではできない / 高くつく」こと

2026-11-23、master `e9bba35` 時点。RubyVM のような処理系固有の機能は対象外。
出典は [unsafe_catalog.md](unsafe_catalog.md)（以下 UC）、[safe_heap.md](safe_heap.md)、[safe_stack.md](safe_stack.md)、
[io_design.md](io_design.md)、[todo.md](todo.md)、`~/ruby/src/trials/` の各 trial（以下 `T:<名前>`）、
`~/ruby/src/ideas/`。

表記:

* **[実測]** = このプロジェクトで計測・実験した（出典つき）。**[実験]** = この文書のために小さく試した（末尾「この文書の実験」）。
  **[未検証]** = 理屈・文献だけで、試していない。
* 数字はすべてベンチ 1 本ごと。命令数は `instructions:u`（ローカル）、時間は sp4。
* 比較相手の C 実装: **koruby_precise**（`~/ruby/astro/sample/koruby_precise`、単体の処理系、以下 kp）と
  **abruby**（`~/ruby/astro/sample/abruby`、CRuby の C 拡張として動き GC・Fiber は CRuby のものを使う）。

## 要約

| 区分 | 項目 |
|---|---|
| A. Rust ではできない（Rust の外に出る必要がある） | A1 callcc / Continuation、A2 CRuby 互換の C 拡張 API（可変長引数の `rb_*` と `rb_raise` の longjmp）、A3 setjmp/longjmp による非局所脱出全般、A4 ガードページの SIGSEGV から SystemStackError に戻る |
| B. 取り除けない `unsafe` が要る | B1 Fiber のスタック切り替え、B2 GC（保守的スタック走査・ヒープの土台・`Value` を語から作る）、B3 コンテキストは同時に 1 つ（`Token::mint`）、B4 独自アロケータ、B5 libc / OS インターフェース（fork・signal・setenv）、B6 mmap（IO::Buffer）、B7 焼いたコードの読み込みと patch 束縛（memfd RW/RX・dlsym・穴の `asm!`）、B8 `$0 =`（argv 領域の書き換え）、B9 Kernel#syscall |
| C. safe にできるが測れる代償がある | C1 値スタック、C2 `Value::fix` など、C3 `ConstCache` の Option、C4 `L_::nth` の `unreachable_unchecked`、C5 Env の検査、C6 ivar の種類検査、C7 VM 大域の `unwrap`、C8 オブジェクトごとの RefCell、C9 VM の表の RefCell、C10 `with_heap` クロージャ、C11 rclass 除去・NO_CTX（`c` の引き回し）、C12 cdylib の thread-local、C13 placement new が無い、C14 小さい文字列バッファ SBuf、C15 シンボル表の Mutex、C16 `Block` が `Copy` なので `Cell` にできない、C17 フレーム連鎖・ノードグラフ（未計測）、C18 安定 ABI が無い・rustc の焼き時間 |
| D. Rust のせいで残した CRuby との違い / Rust で楽になったこと | 違い: `$0 =` が ps に出ない、Continuation 無し、C 拡張無し、IO::Buffer で CRuby が解放済みメモリを触る所は例外。楽になった: 借用検査が chomp/split の不具合を拒否、`Blk<'call>` でブロックの逃がしがコンパイルエラー、`Send + 'static` がワーカの規約を検査、io_uring のバッファを所有権で表す（設計のみ）、ニッチ最適化で 0 コストの unsafe 除去、`MaybeUninit` を構造体リテラルに。両面: `&mut` の noalias がインラインキャッシュの設計の穴を実際の誤コンパイルにした、`str` の UTF-8 不変条件 |
| Rust 固有ではないもの | CRuby がビルドした `.so` をそのまま読む、Fiddle（libffi）、Ractor の並列実行、ネイティブスレッド |

## A. Rust ではできない（Rust の外に出る必要がある）

### A1. callcc / Continuation

* **内容**: CRuby の `callcc` はマシンスタックを丸ごと写し、後で書き戻して `setjmp` の地点にもう一度戻る。
  一度戻った（あるいは捨てた）フレームを何度でも再開する。
* **Rust が障害になる理由**: Rust のフレームは所有する値（`Box`、`Vec`、`RefCell` の借用ガード）を持ち、
  ドロップは 1 回だけ走る前提で、借用規則も「フレームは 1 回だけ戻る」ことを前提にしている。
  写しておいたフレームを再開すると、同じ値を 2 回ドロップし、すでに終わった借用を復活させる。
  `unsafe` でもこれを定義された動作にする方法は無い。さらに `setjmp` のような「2 回戻る」関数を呼ぶための属性
  （`ffi_returns_twice`）が stable にも nightly にも無い **[実験 1]**。
* **saruby の現状**: 実装していない（`src/` と `prelude/` に `callcc` / `Continuation` は無い）。
* **C 版**: kp にも無い（`lib/ruby/continuation.rb` は `NotImplementedError` を投げる）。したがって比較では差になっていない。
  C なら CRuby と同じやり方（スタックの写しと `setjmp`）を書けるが、kp は木を辿る評価器と RESULT による脱出の設計上、作っていない。
* 代わりに一回きりの継続は Fiber で書ける（B1）。

### A2. CRuby 互換の C 拡張 API（ソースを再コンパイルする層）

* **内容**: kp は「Track A」として CRuby の `ruby.h` 互換層を C で持ち、`ext/digest` と `ext/openssl` を無改造でビルドして
  CRuby の test/digest・test/openssl を CRuby と同じ件数で通している
  （`~/ruby/astro/sample/koruby_precise/docs/c_ext_api_design.md` §9）。saruby には無い。
* **Rust が障害になる理由**（2 つとも言語の規則）:
  1. `rb_funcall(recv, mid, n, ...)`、`rb_raise(klass, fmt, ...)`、`rb_scan_args`、`rb_sprintf` などは C の可変長引数関数。
     Rust は可変長引数関数を**呼べる**が、**定義する**のは unstable（`E0658: C-variadic functions are unstable`）**[実験 1]**。
  2. `rb_raise` / `rb_protect` / `rb_ensure` は拡張の C フレームを越えて戻る非局所脱出で、kp は拡張の入口ごとに `setjmp`、
     `rb_raise` で `longjmp` している（同 §9 の 6）。Rust からは `setjmp` を健全に呼べない（A1、A3）。
  よって入口と可変長引数の関数は C の shim にする必要がある。shim を使えば作れると考えているが、**[未検証]**（作っていない）。
* **saruby の現状**: C 拡張は無い。影響: rubyspec library/openssl の 173 件（todo.md「library/ の残り」）、
  ruby-bench の `ractor-conservative-stack`（mkmf で C 拡張をビルドして require する、todo.md、`T:2026-11-21-saruby-ruby-bench`）。
  zlib・psych・bigdecimal・digest などは Rust に移植して埋めた（todo.md「library/」）。
* **C 版**: kp はできている（上記）。ただし CRuby 用にビルドした `.so` を**そのまま**読むことは kp もできない
  （オブジェクトの配置が CRuby のものなので、「Rust 固有ではないもの」）。

### A3. setjmp / longjmp による非局所脱出

* **内容**: CRuby は `raise` / `throw` / `break` を `setjmp` / `longjmp`（EC_JUMP_TAG）で実装する。
* **Rust が障害になる理由**: `setjmp` は「2 回戻る」関数で、Rust にはそれを宣言する手段が無い **[実験 1]**。
  `longjmp` で Rust のフレームを飛び越すのは、そのフレームにドロップの残りが無い場合に限って許されるという扱いで、
  一般のコードでは守れない **[未検証: 規則の細部は Rust の仕様が固まっていない]**。
* **saruby の現状**: `Result<Value, Esc>`（`Esc` は大きさ 0、中身はコンテキスト）で戻り値として伝える（`README.md`「How it differs from ASTro」）。
* **C 版**: ASTro 側も同じ方式（`RESULT` 構造体、abruby `context.h:66-82`）なので、**この選択による代償は Rust 由来ではない**。
  Rust で問題になるのは A2 の C 拡張の境界だけ。

### A4. ガードページの SIGSEGV から SystemStackError へ戻る

* **内容**: CRuby はスタック深さの比較に加えて、ガードページへの SIGSEGV をシグナルハンドラ（sigaltstack）で受けて
  SystemStackError にする。
* **Rust が障害になる理由**: シグナルハンドラから Rust のフレームを越えて戻るには `longjmp` か巻き戻しが要り、
  どちらもできない（A3）。std 自体も main スレッドのスタックあふれに SIGSEGV ハンドラを入れて、メッセージを出して abort する。
* **saruby の現状**: 呼び出しごとの明示的な比較（`vm.rs:1401`、`vm.rs:2206`）。行外のエントリ呼び出し 1 回あたり 3 命令
  （`lea`・`cmp`・`ja`）**[実測]**（`T:2026-11-21-saruby-call-path-2` README「Native stack check」、
  `T:2026-11-21-saruby-recursion` の表で「guard page + signal なら 0、SystemStackError を rescue できるので比較を残す」）。
  ガードページ方式は「シグナルハンドラが要る（unsafe）」として試していない（UC 2.3 の却下表）。
* **C 版**: CRuby も比較を併用している。差は 3 命令/呼び出しを消せるかどうかだけ（消せるかは **[未検証]**）。

## B. 取り除けない `unsafe` が要る（essential unsafe）

数は UC 1.1・1.2（手書き 342、生成コード 93、`d983d25` + UC 4.4 時点）。`src/builtins/` は `sys.rs` 以外
`#![forbid(unsafe_code)]`。以下の「C 版」は、多くの項目で C も同じ操作（asm・mmap・FFI）を書いており、
**差は「C ではそれが普通のコード、Rust では `unsafe` と書いて契約を述べる」ことだけ**、という点に注意。

| # | 項目 | なぜ safe Rust では書けないか | saruby の現状 | 代償・発見 | C 版 |
|---|---|---|---|---|---|
| B1 | Fiber のスタック切り替え | マシンスタックの切り替えは Rust の抽象機械の外。naked 関数とその asm は定義上 `unsafe`。中断したスタックの上に生きている参照があることを型で言えない | `fiber_os.rs` 22 + `fiber.rs` 1（`fiber_os.rs:120`、`:134` の `#[unsafe(naked)]`）。スタックは mmap + ガードページ | [実測] resume+yield 1 往復 403 cycles（旧設計 477）、`T:2026-11-21-saruby-fiber`。safe な代替（OS スレッド 1 本 / ファイバー、async 化した評価器）は [未検証] | kp も `swapcontext` + asm（`builtins/fiber.c:88`、`:187`）。abruby は CRuby の Fiber。CRuby も asm の coroutine |
| B2 | GC: 保守的スタック走査、`Value` を語から作る、ペイロードの参照 | オブジェクトのグラフは Rust の値でなくコレクタが所有し、`Value` は語。スタックの任意の範囲を `&[usize]` として読む操作は Rust に無い | `gc.rs` の `unsafe impl GcLang`（`gc.rs:275`）、`Value::from_raw`（`value.rs:30`）、`heap.rs` の `body` / `body_mut`。GC & ヒープの土台 61 | — | CRuby も保守的走査（C）。kp は正確 GC（スロットで root）なので保守的走査は不要 |
| B3 | 「動くコンテキストは同時に 1 つ」（H2） | 実行時の性質で、型では言えない | `heap::Token::mint`（`heap.rs:20`）が `Ctx` を作る唯一の口。呼ぶのは main と各ファイバーの 2 か所 | — | C では規約（kp は「グローバル禁止」の規則と CodeQL） |
| B4 | 独自アロケータ | `GlobalAlloc` は `unsafe trait` | `kalloc.rs:90`（13） | [実測] 外すと nesteddata +98.6 % / +141.4 %、rpartition_10 +66.7 % / +80.2 %、gen_gc +31.2 % / +49.7 % 命令（interp / AOT、UC 4.2、ideas `rejected/saruby-drop-kalloc`） | C も独自アロケータを書く。Rust 特有なのはスレッドポインタを `asm!` で読む部分（C12） |
| B5 | libc / OS インターフェース | FFI 呼び出しは `unsafe`。ラッパーで義務を消せないものが残る: `fork`（子は直ちに exec か単独で動く）、`setenv`（他スレッドの `getenv` と競合。Rust 2024 では `std::env::set_var` 自体が `unsafe`）、`signal` のハンドラ契約、`_exit` | `builtins/sys.rs`（114、1 ラッパー 1 個、`SAFETY` 付き） | [実測] fork 後の不具合 1 件: patch ローダの Arena が MAP_SHARED の memfd を `&'static mut [u8]` として持ち、fork 後に親子が同じ `&mut` で書いた（knucleotide が SIGSEGV、`T:2026-11-22-saruby-knucleotide-segv`、UC 2.4「found later」）。論理的な誤りは C でも同じ | 同じ FFI を書く |
| B6 | mmap（IO::Buffer） | 写像したメモリをスライスとして見せるのは `unsafe`。MAP_SHARED のファイルを他プロセスが書く・切り詰めると SIGBUS で、Rust の参照の前提を外から破れる | `sys::Mapping`（`sys.rs:473`）の 5 個。`io_buffer.rs` 本体は `forbid`、アドレスは比較だけで参照外ししない（io_design.md「IO::Buffer」） | rubyspec core/io/buffer 198 / 198 | CRuby・memmap2 も同じ義務 |
| B7 | 焼いたコードの読み込み、patch 束縛 | 機械語を書いて実行する。`dlsym` のアドレスを関数ポインタへ `transmute`。穴（行番号）を即値の再配置にする命令は `asm!`（`sym` オペランド）でしか書けない | `runtime/astrust.rs`（dlopen / dlsym 5）、`runtime/astrust_loader.rs`（memfd RW/RX 10、`:282`）、`aot.rs`、生成される hole item の `asm!` | [実測] optcarrot −0.83 % 命令/フレーム、tak −6.3 %、RSS +9 MB（`T:2026-11-21-astrust-patch-binding`）。rustc の PIC は全部 GOTPCREL を出し、`-C relocation-model=static` は libsaruby のデータまで 32 bit 即値にするので使えない（同 README） | ASTro も穴を asm で書く（`~/ruby/astro/runtime/loader/arch_x86_64.h:54`「The asm is not decoration」）。Rust 固有なのは relocation model をシンボル単位で選べない点 |
| B8 | `$0 =` / setproctitle（argv 領域の書き換え） | std は argv を所有せず、取る API も無い。`.init_array` に関数を置けば glibc から argv を受け取れ、その領域は書ける（`unsafe`）。ただし std の `env::args()` は glibc 上で argv を遅延して読むので、書き換え後の `env::args()` は書き換えた内容を返す（std が不変と仮定している領域を書く） | 何もしない（`builtins/process.rs:37` の `set_proctitle` は空）。一時 `/proc/self/comm` を書いたが CRuby と逆の振る舞いなので外した（15ad997） | [実験 3] で書き換えと `/proc/self/cmdline` への反映を確認。todo.md の「Rust が持たないメモリへの書き込みが要る」は「`unsafe` と std の前提との衝突」と読むのが正確。environ 領域まで広げる CRuby の処理は [未検証] | C では `main(argc, argv)` から普通に書ける |
| B9 | Kernel#syscall | 整数を任意のポインタとして渡すので本質的に unsound。Rust は可変長の `syscall(2)` を呼べる（`sys.rs:68` で宣言済み） | 実装していない | [未検証] | C でも同じく unsound |

## C. safe にできるが、測れる代償がある

「採否」は現状の選択。**kept unsafe** = 代償を避けて unsafe を残した、**accepted** = 代償を払って safe にした。

| # | 項目 | Rust の何が理由か | 代償（[実測]） | 採否 |
|---|---|---|---|---|
| C1 | 値スタック | 呼び出し元の積んだ引数が呼ばれた側のローカルになる（エイリアス）。生ポインタ無しでは `Cell<Value>` スロット + 範囲検査 | 導入時 AOT fib +7.1 %、tak +17.1 %、collatz +7.5 % 命令。添字で引く `emu_idx` 形は +16〜89 %（毎呼び出しで基底と長さを読み直す）。`f88d3e6` で呼び出しの多いベンチ +3〜13 % 命令だが、sp4 の時間では fib・tak・method_call・send が同等か速く、collatz は safe 側が −22 %。長いベンチ 53 本で safe が遅い 20 / 速い 15 / 重なる 19（UC 3、safe_stack.md） | accepted（既定は safe。`FEATURES=raw_stack` で比較用） |
| C2 | `Value::fix` / `sym` / `flonum` を `NonZeroU64::new(..).unwrap()` で | `NonZeroU64::new_unchecked` 以外では検査が残る | 定数の `Value` と LitCache は 0 コストで外せた。fix/sym/flonum は hash +0.6〜1.4 %、nesteddata +7 % 命令（他関数のレジスタ割り当てが動く、UC 4.4） | kept unsafe |
| C3 | `ConstCache.val` を `Option<Value>` に | 同上（読みごとに 1 検査） | interp hash +0.28 %、str +0.28 %、gcchurn +0.20 %、object +0.12 %（UC 1.3） | kept unsafe（`vm.rs:256` は `Cell<u64>`） |
| C4 | `L_::nth` の既定アーム | 添字が定数でない所で範囲検査が残る | optcarrot の SD 377640 → 399961 バイト、all.so .text +1.04 %、+0.07 % 命令/フレーム（UC 8.2） | kept unsafe（`lib/astrustgen.rb:1003`） |
| C5 | Env の読み書きの種類検査 | 「フレームの env スロットは Env」を型で言えない | 検査つき `c.env_*` で block AOT +29 %、iterators AOT +25 % 命令（UC 7「object accessors」） | kept unsafe（`heap.rs:386` `env_*_unchecked`） |
| C6 | ivar 読みの本体の種類検査 | 「キャッシュに当たったクラスのインスタンスは素のオブジェクト」を型で言えない | 検査を外すと optcarrot −2.8 % 命令/フレーム | accepted（unsafe 案は却下、ideas `rejected/saruby-ivar-kind-check-unsafe`） |
| C7 | VM 大域の `unwrap_unchecked` → `unwrap` | `static` の初期化済みを型で言えない | str AOT +2.8 %、intdiv AOT +1.9 %、strfmt +1.6 / +1.8 %（unsafe は 1 つも減らない、ideas `rejected/saruby-vm-global-checked-unwrap`） | kept unsafe |
| C8 | ヒープのペイロードをオブジェクトごとの RefCell 風の借用フラグで | 共有と変更の規則を実行時に数える | ary AOT +96 % cycles、sieve +85 %、fannkuch +66 %（safe_heap.md「Alternatives」）。代わりに「`Ctx` を借りるアクセサ」（`c.ary(v)`）で種類検査だけ: 0〜+2.4 % 命令（UC 6） | RefCell は却下、アクセサを採用 |
| C9 | VM の表（mcache、globals など 22 個）を RefCell | `&'static mut Vm` をやめて内部可変に | 15 本中 13 本が 0.2 % 以内、poly +0.13 % / +0.29 %（mcache の RefCell と推定、[未検証]）（UC 7「vm / heap split」） | accepted |
| C10 | `with_heap(\|h\| ..)` クロージャで `&'static mut Heap` を消す | クロージャが AOT 側の crate で inline されず、確保 1 回ごとの呼び出し + 本体のスタック経由コピー | gcchurn AOT 988M → 1212M（+23 %）、`#[inline(always)]` でも str +0.85 % / +1.4 %。文の終わりで借用が切れる `heap(&mut Scope)` 形にして命令数・機械語とも base と同一（UC 7「gc heap」） | 0 コストの別形で解決 |
| C11 | 文脈 `c` を全関数に引き回す（rclass 除去、NO_CTX） | `&'static mut` を返す安全関数（unsound）をやめると、借用のために `c` を渡し続ける必要がある | rclass 除去: poly +0.85 % / +1.17 %、object +0.3 %、optcarrot +0.36 %/フレーム（`call_slow` でレジスタ移動 ~2.7 命令/呼び出し）。NO_CTX: hash AOT +1.4 %、nesteddata AOT +1.0 %、object AOT +0.62 %（UC 7、todo.md 性能） | accepted（取り戻しは未着手） |
| C12 | cdylib / dylib の thread-local | stable rustc は TLS モデルを選べない（`-Z tls-model` は nightly のみ）。共有ライブラリの `thread_local!` は毎回 `__tls_get_addr` の呼び出し。C は `__attribute__((tls_model("initial-exec")))` で `%fs:` 1 回 **[実験 2]** | blop の状態を `thread_local!` + RefCell にした版で vm_thread_condvar1 / sized_queue4 の命令の 16〜20 %（`T:2026-11-22-saruby-blop`）。error_highlight の初版で起動 +2.8 %、m_raise +8 / +15 %（`T:2026-11-23-saruby-error-highlight`） | 回避: 状態を `Vm` / static に移した。kalloc は `asm!` で `fs:[0]` を読む（`kalloc.rs:28`、[未検証]: `thread_local!` にした場合のコスト） |
| C13 | placement new が無い | 本体を値で作って確保関数に渡すと、スタックの一時領域から写すコピーになり、ストアフォワーディングで待つ | sp4 gcchurn 460M → 725M cycles、strscan −3 % 命令で +30 % cycles。確保が決まってからスロット内で組み立てる `alloc_with`（`gc.rs:76`）で 461M に戻した（ideas `shipped/saruby-alloc-with`、UC 2.4 末尾） | 回避（クロージャで組み立て） |
| C14 | 小さい文字列のバッファ SBuf（24 バイト、23 バイト inline） | inline 配列と `Vec` の union を空きビットで判別する形は safe な enum で同じ大きさにできない | safe 版 3 種を計測、最良の rs29 で str +4.3 % / +6.1 %、strscan +9.4 %、to_words_10 +7.8 % 命令（UC 1.3・4.2、ideas `rejected/saruby-sbuf-safe-rs29`） | kept unsafe（`sbuf.rs` 10） |
| C15 | シンボル表を `static Mutex` | `static mut` を避ける一番素直な形 | シンボルの多いループ +8 % 命令。lock-free な名前表 + `AtomicU32` のキャッシュで symbench −9 % / −15 %（UC 2.2 wave 3、ideas `rejected/saruby-symtab-mutex`） | Mutex は却下、atomic で safe |
| C16 | `block_to_proc` が呼び出し元の `Block` に `*const` 経由で書く | `Block` は `Copy` で、`Cell` は `Copy` でないので `proc_val` を `Cell` にできない。共有参照由来のポインタへの書き込み | 計測なし（直す形を作っていない、[未検証]） | 残している（`vm.rs:2398`、todo.md） |
| C17 | フレーム連鎖、ノードのグラフ | フレームはネイティブスタック上にあり `prev` で繋がる（スタックを通した自己参照）。ノードはアドレスが識別子（焼いたコードは `root + 定数`）。どちらもライフタイムで言えない | safe 形（`Vec` + 添字）は未計測。値スタックの `emu_idx`（+16〜89 %）が一番近い数字（UC 5）。ノードの添字アリーナは焼いたコードのアドレス計算を壊す | kept unsafe（呼び出し経路 144、生成コード 93 は監査済みの 13 プリミティブに集約、UC 8） |
| C18 | 安定 ABI が無い、rustc が遅い | 焼いたコード（`all.so`）は Rust の dylib である libsaruby に `--extern` でリンクし、inline も跨ぐ。同じ rustc・同じビルドでしか使えない（ビルド id で照合、`runtime/astrust.rs:192`）。`--emit=obj` は codegen-units が 1 に戻る | 焼き時間: ASTrust 0.2〜1.7 s / プログラム、ASTro（gcc）~0.2 s（`T:2026-11-20-astrust-vs-astro`）。optcarrot 47 s、30k_* は rustc がメモリ不足（todo.md 性能） | accepted |

0 コストで外せたもの（代償なし、[実測] UC 4.4）: 定数 `Value` を const fn、`LitCache` を `Option<Value>`、
`static mut` のフラグを atomic（REFINEMENTS_ON、BOOTED、REDEF ほか）、`CallCache.cfunc` を `Option<fn>`、
`aot::STORE` を `thread_local!`（使用頻度が低い）。371 → 342。

## D. Rust のせいで残した CRuby との違い / Rust で楽になったこと

### 残した違い

| 違い | 理由 | 出典 |
|---|---|---|
| `$0 =` が ps のコマンド行に出ない（`$0` の値は変わる） | B8。作れるが std の前提との衝突を引き受けていない | `process.rs:36-37`、todo.md |
| `Continuation` / `callcc` が無い | A1 | — |
| C 拡張を require できない（openssl 173 件、ractor-conservative-stack） | A2。主要な ext は Rust に移植 | todo.md |
| IO::Buffer で CRuby 4.0.2 が解放済み・範囲外のメモリを触る 6 ケースは例外を出す（`to_int` が解放する、`each` のブロックが解放する、`and!` の不正なマスクなど） | `io_buffer.rs` は `forbid(unsafe_code)` で、アドレスを参照外しせず、境界検査つきで今のメモリから取るため。安全側を選んだ | io_design.md「IO::Buffer」、`T:2026-11-23-saruby-io-buffer`（deviations.rb） |

### Rust で楽になったこと

* **借用検査が不具合を拒否した**: `chomp` / `chomp!` / `split` が受け手のバイトを `to_str` のまたぎで持っていた
  （CRuby 303、旧 saruby 3）。変換後の最初の `cargo check` が E0502 を出した（UC 2.4 行 7）。
  ただし **49 件中、借用検査で見つかったのは 1 件だけ**（実行時検査 1、読み直し 25、テスト 22）。長さ・添字・`Copy` の
  スナップショットを借用の外に写すと、同じ誤りでもコンパイルが通る（UC 2.4.1-2.4.2）。
* **`Blk<'call>`**: 組み込みメソッドに渡すブロックをライフタイム付きの型にしたので、ブロックのアドレスを整数にする
  （E0605）、ライフタイム無しで構造体に入れる（E0106）、`'static` に置く（E0521）がコンパイルエラーになる。
  以前の SIGSEGV（UC 2.4 行 45、46。行 46 の形は CRuby 4.0.2 も segfault）は今なら型エラー。代償は 15 本で ±0.001 % 命令（UC 2.4.3）。
* **`Send + 'static`**: ワーカースレッドに渡すクロージャ（getaddrinfo）が VM に触らないことをコンパイラが検査する。
  koruby では CodeQL の規則が要る（io_design.md「CFUNC」）。
* **io_uring のバッファ（設計のみ）**: カーネルが持っている間のバッファを slab の所有にでき、呼び出し側に返せないことを型で言える（io_design.md「Engines」、[未検証]）。
* **ニッチ最適化**: `NonZeroU64`、`Option<fn>`、`Option<NonZero>` は unsafe 版と同じ 1 語なので、safe 版の大きさが同じ（C の 0 コスト除去の大半）。
* **`MaybeUninit` を構造体リテラルに**: boot が後から足したフィールドを未初期化のまま残す潜在 UB が、今は E0063（UC 2.4 行 39）。
* **debug ビルドの整数あふれ検査**: `[:a, :b].cycle(2**62).size` のあふれを見つけた（UC 2.4 行 41-43）。

### 両面（Rust の規則が、C には無い問題を起こした）

* **`&mut` の noalias**: ノードの本体がインラインキャッシュへの `&mut` を呼び出しのまたぎで持ち、再帰で同じノードに再入すると
  `&mut` が 2 つになった。規則違反なだけでなく、LLVM が noalias を使って呼び出し前の `cc.klass` を再利用し、AOT で
  A のインスタンスに B#f を実行した（`-Zmutable-noalias=no` で正しくなることで原因を切り分け、UC 2.4 行 44・8.4、
  `T:2026-11-23-saruby-ic-borrow`）。ASTro の C は素のポインタなのでコンパイラは呼び出しが書きうると仮定し、この誤りは起きない。
  修正は全キャッシュを `Cell` フィールドにして本体に `&T` を渡す形で、型で健全になった（block AOT +3.65 % 以外は同等か減）。
* **`str` の UTF-8 不変条件**: Ruby の文字列はエンコーディング付きのバイト列で、Rust の `str` には載らない。
  `from_utf8_unchecked` を latin-1 のパスに使っていた UB（UC 2.4 行 26）。saruby の文字列は自前の `SBuf`（C14）。

## Rust 固有ではないもの（C でも同じ）

* **CRuby がビルドした `.so` をそのまま読む**: 拡張は CRuby のオブジェクト配置（`RSTRING_PTR` などのマクロ）でコンパイルされている。
  kp も再コンパイルを要求し、CRuby 用の `.so` は名前（`.kompat.so`）と ABI 印で拒否する（c_ext_api_design.md §9）。
* **Fiddle**: 実行時に決まるシグネチャで呼ぶには libffi か asm が要る。C も同じ（CRuby は libffi）。
* **Ractor の並列実行、ネイティブスレッド**: saruby は全 Ractor・全スレッドを 1 つのインタプリタスレッド上の緑スレッドで動かす
  （ractor.md、design.md「Threads」）。`Vm` のフィールドは `Cell` / `RefCell`（`Sync` でない）なので並列化には設計変更が要るが、
  それを Rust の制約と呼べるかは評価していない（**[未検証]**: 並列化を試していない）。

## この文書の実験

rustc 1.97.1（stable、リポジトリと同じ）、gcc 15.2。scratch で行い、削除済み。

1. C 可変長引数関数の定義 `pub unsafe extern "C" fn rb_funcall(recv: u64, mid: u64, n: i32, mut args: ...)` →
   `error[E0658]: C-variadic functions are unstable`。`extern "C" { #[ffi_returns_twice] fn _setjmp(..) }` →
   `cannot find attribute 'ffi_returns_twice'`。
2. cdylib の `thread_local! { static X: Cell<u64> }` を増やす関数: `call __tls_get_addr@plt` を含む 7 命令。
   `-Z tls-model=initial-exec` は「only accepted on the nightly compiler」。同じ関数を C（`-fPIC -shared`、
   `tls_model("initial-exec")`）で書くと `mov %fs:(%rdx),%rax` / `add` / `mov` で呼び出し無し。
3. `$0 =`: `#[link_section = ".init_array"]` の関数で glibc から argv を受け取り、`unsafe` で argv 領域を
   `"saruby: renamed"` に書き換えた。`/proc/self/cmdline` は書き換え後の内容になり、`std::env::args()` も
   `["saruby: renamed", " renamed", ""]` を返した（std が argv をコピーせず遅延して読むため）。
