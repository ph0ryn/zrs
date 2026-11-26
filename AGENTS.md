# AGENTS.md

`zrs` の設計・実装・検証に着手する際の初期判断に必要な前提と確認先をまとめる。各領域の詳細は正本文書とコードを参照する。

## 目的と対象外

`zrs` は Zsh 専用の入力支援・補完ツールであり、YAML の宣言的設定で略語展開・スニペット挿入・カスタムファジー補完を提供する。履歴選択は Zsh の履歴を fzf で絞り込み、設定を参照せずにバッファへ復元する。

以下は解決範囲に含めない。

- Zsh 以外のシェル（Bash / Fish 等）向けの対応や抽象層
- 動的設定や組み込みスクリプトエンジン（Rhai / Lua / WASM 等）
- コマンド履歴の独自管理（保存・管理は Zsh 自身に委譲する）
- ファジー検索 UI の独自実装（`fzf` に委譲する）

## 参照基準（正本）

変更前に該当する正本を確認する。

- [docs/spec.md](docs/spec.md): 外部から観測可能な仕様・設定スキーマ・文脈ガード境界・既定キーバインド
- [docs/architecture.md](docs/architecture.md): 2 層構成・実行モデル・CLI/ZLE 境界プロトコル・技術選定
- [docs/verification.md](docs/verification.md): 検証の層と機能・境界ごとの合格基準
- [docs/development.md](docs/development.md): Nix 開発環境・ワークフロー・配布構成
- [Cargo.toml](Cargo.toml), [flake.nix](flake.nix), [justfile](justfile): 依存関係・パッケージ構成・実行コマンドの定義

## 責務と依存の境界

- **ZLE ラッパー（[shell/zrs.zsh](shell/zrs.zsh)）と Rust CLI（[src/](src/)）の分離**:
  - ZLE 側はキー捕捉・`BUFFER`/`LBUFFER`/`RBUFFER`/`CURSOR` の読み書き・`${(z)LBUFFER}` トークン抽出・既存ウィジェットへのフォールバック・`zsh/parameter` による `$history` レコードの提供のみを担う。ZLE 側にビジネスロジック、`jq` 依存、`eval` を持ち込まない。
  - Rust 側は設定読み込みと検証・ルール評価・テンプレート処理・外部コマンドおよび `fzf` 呼び出しを担う。Rust 側で Zsh の字句・構文パーサーを全面再実装せず、`${(z)LBUFFER}` の結果と未完成構文ガードを組み合わせて扱う。
- **型と共通処理の境界**:
  - `abbreviations` と `snippets` は設定スキーマ・型を分離したまま保ち、プレースホルダー解析とカーソル位置計算は [src/template.rs](src/template.rs) に集約する。
- **配布と Flake 依存の境界**:
  - [shell/zrs.zsh](shell/zrs.zsh) は `include_str!` でバイナリに埋め込み、単一バイナリ配布を維持する。
  - [flake.nix](flake.nix) の外部入力は `nixpkgs` のみに保つ（配布時の依存グラフ最小化のため）。

## 制約と判断理由

- **同期ワンショット実行を維持する**: 非同期ランタイム（Tokio 等）や常駐サーバーは導入しない。応答性が実用基準を満たさないと実測で判明した場合にのみ、キャッシュや常駐化を検討する（[docs/architecture.md](docs/architecture.md)）。
- **設定パス解決に OS 既定ディレクトリ系クレートを使わない**: `dirs` 等は macOS で `~/Library/Application Support` を返し、`$XDG_CONFIG_HOME` / `~/.config` の仕様に反するため、環境変数から直接解決する。
- **YAML パーサーは `serde-saphyr` を用いる**: `serde_yaml` / `serde_yml` は非推奨のため採用しない。未知フィールドを黙殺せず `#[serde(deny_unknown_fields)]` で拒否する。
- **エラーを黙示的な既定値で隠さない**: YAML 構文誤り・スキーマ違反・不正正規表現・外部コマンド失敗は明示的にエラーを返し、ZLE 側でバッファを破壊しない。例外として、対話コマンド実行時の設定ファイル未存在のみ空設定として扱う（`zrs validate` はエラーとする）。
- **ZLE 境界の正確性と共存を保つ**:
  - Rust 内部の UTF-8 バイト位置は ZLE の文字単位 `CURSOR` へ変換して返す。
  - 補完候補の挿入時は Zsh 向けクォーティングを施す。
  - `Space` / `Enter` / `Tab` の既存バインド先を退避・呼び出しできるようにし（`^R` も退避対象に含め）、再初期化時に自前ウィジェットで退避先を上書きしない。
  - `fzf` 起動時は結果返却用の標準出力と制御端末（`/dev/tty`）を干渉させない。
- **安全性とビルド整合性**:
  - [Cargo.toml](Cargo.toml) の `unsafe_code = "forbid"` を維持する。
  - ビルド・テスト対象のディレクトリやファイルを新設した場合は、[nix/package.nix](nix/package.nix) の `lib.fileset` を更新し、Git 追跡対象に含めてから `nix flake check` で検証する。
