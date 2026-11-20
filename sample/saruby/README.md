# saruby — Sort of A Ruby

**ASTrust**（AST を辿るインタプリタを部分評価して速くするフレームワークの Rust 版）の上に作った Ruby 処理系です。
C で書かれた [astro/sample/koruby_precise](https://github.com/ko1/astro) を Rust で書き直したもので、目標は 2 つあります。

- **CRuby の drop-in 互換**を目指すこと。
- **safe Rust でどこまで C 実装に迫れるか**を確かめること。`unsafe` は外部とのインターフェースと、小さく監査できる中核だけに閉じ込めます（Trusted Ruby）。

> 実験中です。動かないものはまだ多く、内部の構成も頻繁に変わります。

## 使い方

必要なもの: 安定版の Rust、Ruby 3.4 以降（ジェネレータと、純 Ruby の標準ライブラリの参照元）、C コンパイラ（同梱の prism をビルドします）。

```sh
make                              # node.def -> src/node_*.rs（astrustgen）-> cargo -> ./saruby
./saruby -e 'puts "hello"'
./saruby prog.rb                  # 焼いたコードがあれば使い、なければインタプリタで実行
./saruby --plain prog.rb          # 常にインタプリタで実行
./saruby --aot-compile prog.rb    # 実行してから、その AST を特殊化して rustc でコンパイル（code_store/ に保存）
./saruby prog.rb                  # 2 回目以降は焼いたコードで実行
```

| ビルドの切り替え | 意味 |
|---|---|
| `make`（既定） | 値スタックは unsafe なしの実装（`src/stack.rs`、`#![forbid(unsafe_code)]`） |
| `make FEATURES=raw_stack` | 値スタックを生ポインタ版にする（`src/stack_raw.rs`）。safe のコストを測る比較用 |
| `make GC=marksweep`（既定）/ `make GC=bitmap` | GC バックエンドの選択（`../../docs/gc.md`） |

| 環境変数 | 意味 |
|---|---|
| `SARUBY_GC_STRESS=n` | n 回の確保ごとに GC を走らせる（ルート漏れの検査） |
| `SARUBY_SD_NAMES=1` | 焼いたコードの関数名 `SD_<hash>` がどのメソッド・ブロックかを表示する（プロファイルを読むため） |
| `SARUBY_AOT_VERBOSE=1` | 焼くときの rustc の出力を表示する |
| `ASTRUST_SPEC_DEPTH=n` | 呼び出し先を呼び出し箇所に組み込む深さ（既定 2、0 で無効） |

## 仕組み

- **パーサ**: prism（C、同梱）。直列化した AST を Rust 側で読み、`node.def` のノード木に変換します（`src/compile.rs`）。
- **ノード**: 約 100 種類。本体は Rust で `node.def` に書き、ASTrust のジェネレータがインタプリタと部分評価器を生成します。
- **AOT**: `--aot-compile` は、実行して温まったインラインキャッシュを見ながら AST を Rust コードに特殊化します。呼び出し箇所には、そのとき見えていた呼び先（小さいメソッド）を組み込み、箇所の種類（メソッド・属性読み書き・組み込み）に合わせた経路だけを残します。実行中にノードを書き換えることはしません。
- **値**: CRuby 風のタグ付き（fixnum・flonum・シンボルは即値）。オブジェクトは固定長スロットで、ivar は 3 個まで、文字列は 23 バイトまでスロット内に置きます。
- **GC**: 動かさない mark-sweep。ヒープ内の参照とグローバルなルートは正確にたどり、マシンスタックと値スタックは保守的にスキャンします（precise 化は進行中）。
- **Fiber / Thread**: Fiber は専用スタックへの切り替え、Thread はグリーンスレッドです。
- **正規表現**: `../astrogre`（同じく ASTrust 上の実装、Onigmo 互換を目指す）。
- **標準ライブラリ**: 中核は Rust（`src/builtins/`）、一部は Ruby の prelude。純 Ruby のライブラリ（`set`、`optparse`、`fileutils` など）は、ビルド時に記録した CRuby のライブラリディレクトリから読み込みます。

詳しくは `docs/design.md`、今後の方針は `docs/roadmap.md` にあります。

## unsafe の状況

方針は「外部インターフェースと監査済みの中核以外は safe」です。まだ途中です。

- **値スタック**: 既定のビルドは unsafe なし。フレームは `&[Cell<Value>; 65536]` で、焼いたコードでは添字が定数なので境界チェックは消えます（`docs/safe_stack.md`）。
- **オブジェクト本体へのアクセス**: コンテキストを借用するアクセサ（`c.ary(v)` / `c.ary_mut(v)`）に移行中です。本体への参照を持ったままコンテキストを取る関数を呼ぶとコンパイルエラーになるので、別名規則と「確保をまたいで参照を持たない」という GC の規約が、借用チェッカの 1 つの規則になります。配列（`src/array.rs`、`src/builtins/array_methods.rs`）は移行済みで `#![forbid(unsafe_code)]` です。残りは `tools/heap_audit.rb` で数えられます（`docs/safe_heap.md` に手順）。
- **残っているもの**（生成コードを除いて約 570 か所、2026-11-21 時点）: 未移行のアクセサ、GC の中核、マシンスタックの保守的スキャン、prism・libc・`dlopen`・Fiber 切り替えなどの外部インターフェース、ノードのオペランド読み出し。
- **型で保証できないもの**: 「`Value` が指すオブジェクトが生きていること（ルート付け）」は、再帰で木をたどる評価器では型だけでは証明できません。規約と GC ストレスモードで検査します。

## 互換性と性能（2026-11-21 時点）

基準は CRuby です。テストは astro の `sample/rubyharness`（出力を CRuby と行単位で比較）を使っています。

- **t/hand**（1,700 ファイル余り）: 一致 23,069 行 / 不一致 10,419 行。
- **optcarrot**（Lan_Master.nes、180 フレーム）: 動作し、チェックサムは CRuby と一致します。
- **マイクロベンチマーク**（rubyharness の 53 本）: 全本で出力が CRuby と一致します。

速度は、同じ機械・同じ条件で測った実測値をベンチマークごとに記録しています（集合の平均は使いません）。

| optcarrot（fps、5 回の範囲） | |
|---|---|
| CRuby 4.1.0dev | 53.7〜54.7 |
| CRuby + YJIT | 294〜299 |
| CRuby + ZJIT | 311〜316 |
| koruby_precise AOT（ASTro、C） | 313〜321 |
| saruby インタプリタ | 71〜74 |
| saruby AOT（safe、既定） | 125〜129 |
| saruby AOT（unsafe スタック） | 130〜131 |

マイクロベンチマーク 53 本では、saruby AOT（safe）は koruby_precise AOT に対して 33 本で速く、19 本で遅く、1 本は差が計測の揺れの範囲でした（5 回計測の最小〜最大が重ならない場合だけを数えています）。負けているのは、メソッド呼び出しとブロック呼び出しが中心のものです。optcarrot の差も同じ原因で、焼いたコードの大きさ（命令キャッシュミス）が効いています。

## ディレクトリ

```
node.def            ノードの定義（本体は Rust）
saruby_gen.rb       ASTrust ジェネレータの saruby 向け設定
src/compile.rs      prism の AST -> ノード木（ローカル変数の配置、ブロックが捕まえる変数の解析）
src/vm.rs           クラス・メソッド・呼び出し・フレーム・インラインキャッシュ
src/rt.rs           ノード本体から呼ばれるランタイム関数（srb_*）
src/heap.rs         オブジェクト本体への健全なアクセス（ヒープトークン）
src/stack.rs        値スタック（unsafe なし）／ stack_raw.rs（生ポインタ版）
src/gc.rs           GC フレームワーク（../../runtime/gc）への言語側の実装
src/builtins/       組み込みクラス
prelude/            Ruby で書いた組み込み部分
docs/               設計文書
tools/              生成スクリプト、計測用の小道具
vendor/prism/       同梱の prism
```

## 名前について

saruby は "Sort of A Ruby" の略です。ランタイム関数の接頭辞は `srb_`、環境変数は `SARUBY_` です。
