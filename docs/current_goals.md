# 現在の目標（ASTrust / saruby）

作業そのものは `sample/saruby/docs/todo.md` から取る。この文書は、どの todo を先に消化するかを決めるための優先度だけを書く。user の指示で変わったら、ここを直す（会話にだけ残さない）。

## 優先順位（user 2026-11-21）

1. ASTro の原則
2. unsafe を減らす
3. CRuby 互換
4. 性能

## いまの最優先: moving GC

- user 2026-11-25〜26:「full moving 頑張ってほしい」「moving GC はなるべくはやめに終わらせたいね（基本となるところだから）」
- 目標: copy / copy_gen を、正確な root だけで動かす。クラスなどを動かない領域に置くのは可。mostly-copying は最終形にしない。
- 方針（閾値・倍率・昇格年齢・環境変数）は koruby_precise に揃え、比べられるようにする。
- todo では §1.5。設計は `docs/gc_moving.md`、記録は `~/ruby/src/trials/2026-11-25-astrust-moving-gc/`。

## 低い優先度で続けるもの

- カバレッジを埋めるテスト（user 2026-11-26「のんびり継続してください」）: rubyspec → CRuby test/ → 自前、行 → ブランチ → 意味。テストは他の実装でも動くように書く。
