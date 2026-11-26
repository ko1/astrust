# ASTrust / saruby

進め方は ~/ruby/src/agent-process/PROCESS.md に従う（lead・subagent とも §10 は必ず守る）。以下はこのプロジェクトの profile。

## プロジェクト profile

- 目標と優先度: `docs/current_goals.md`（作業は todo から、この優先度で選ぶ）
- 常時の agent 数: 2 本以上
- 照合先（oracle）: CRuby master `~/ruby/install/master/bin/ruby`（koruby と同じ。nightly `~/.local/bin/ruby-master-nightly` が毎朝ビルドし、sp3 / sp4 にも配る）。二番目の確認は 4.0.10
- ビルド構成: `make`（default）/ `make FEATURES=gc_check` / `make GC=bitmap` / `make FEATURES=raw_stack`（sample/saruby で）。release は codegen-units = 1
- 関門（コマンド）:
  - 警告: 各ビルドのログで `grep -c '^warning'` が 0
  - t/hand: rubyharness の `t/hand`（`~/ruby/rubyharness`、Makefile の `-include $(RUBYHARNESS)/harness.mk`）を master と比べる
  - AOT と `--plain` の比較、`reuse_test` / `aot_fork_test`
  - t/cov: `bash ~/ruby/rubyharness/t/cov/run_suite.sh "$PWD/saruby"`（絶対パスで渡す）
  - rubyspec（大きな変更のとき）
  - GC: `make FEATURES=gc_check` + `SARUBY_GC_STRESS=1`（stress が効いていることを GC.count で確かめてから）。`SARUBY_GC_PRECISE_ALL=1` はマシンスタックを走査しない
- 安全性の指標: `ruby tools/heap_audit.rb | tail -n 1` の総数（2026-11-26 時点 396）。増やすときは SAFETY コメントと一覧
- 計測機と排他: sp4（計測対象、専用）/ sp3（比較・負荷生成）。必ず `~/.local/bin/bench-lease claim / check / release`（sandbox の外で）。継続して追う数字は sp4
- 重いジョブのラッパー: `~/.local/bin/saruby-heavy`（空きメモリ ≥10GB で入場、本数の上限なし）。lease を待つコマンドは包まない
- todo: `sample/saruby/docs/todo.md`（テストで出た問題は §3.5）
- 済みの記録（done）: `sample/saruby/docs/done.md`（todo の整理で、済みの項目を元の節見出しの下に新しい順で移す。まだ無ければ作る）
- 実験と台帳: `~/ruby/src/trials/yyyy-mm-dd-saruby-…/`、`~/ruby/src/ideas/`（`ruby index.rb > README.md` で索引を作り直して commit）
- 上流の不具合の記録: `~/ruby/src/trials/2026-11-24-cruby-oddities/`（artifact あり）
- 未マージの記録: `~/ruby/src/unmerged/astrust/yyyy-mm-dd-<slug>/`（git 管理。`~/ruby/src/agent-process/bin/wt-archive` で作る）。tag は ASTrust repo の `unmerged/<slug>`
- worktree: すべて `.claude/worktrees/`（harness の既定）。harness は session の間 agent の worktree を lock するので、終わった agent の分は `wt-reap --apply --force-locked --keep <走っている agent>...` で消す
- 比べたい兄弟の実装: koruby_precise（`~/ruby/astro/sample/koruby_precise`）。呼び名は「saruby (ASTrust)」「koruby (ASTro)」。GC の方針（閾値・倍率・昇格年齢・環境変数）は koruby に揃える
- タイムゾーン: JST（マシンは UTC）
- このプロジェクト特有の注意:
  - 安いモデルの agent には、性能・GC/root の設計・hot path（呼び出し経路・node.def・astrustgen のテンプレート）を渡さない
  - OS やプロセスのメモリとのやり取りは、safe な手段が無ければ unsafe 可（user 2026-11-25）
  - 性能の数字は codegen-units = 1（9d71e1d）以前と以後で別の系列
  - テストは他の実装（CRuby / koruby）でも動くように書く
  - この機械では sandbox が起動しないことがある（nested user namespace）。その場合は sandbox の外で走らせる
