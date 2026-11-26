# Development

`zrs` の開発環境・日常のワークフロー・Nix による配布構成を定める。個別のタスク定義は [justfile](../justfile)、ツールチェーンとパッケージ定義は [flake.nix](../flake.nix) を正本とする。

## 開発環境

Nix Flake の開発シェルに、Rust ツールチェーン（`rustc` / `cargo` / `clippy` / `rustfmt` / `rust-analyzer`）、`zsh`、`fzf`、`hyperfine`、`just` を同梱している。

```sh
nix develop
```

## 日常のワークフロー

主な作業コマンドは [justfile](../justfile) に集約している。

| コマンド | 用途 |
| --- | --- |
| `just check` | フォーマット・`clippy`・Rust テスト・Zsh 構文チェックを一括実行 |
| `just test` | Rust の単体テスト・結合テストを実行 |
| `just lint` | `cargo clippy --all-targets -- -D warnings` を実行 |
| `just zsh-check` | `zsh -n shell/zrs.zsh` で ZLE スクリプトの構文を検証 |
| `just zsh-test` | 擬似端末で ZLE のキー操作と実際の `fzf` 選択を検証 |
| `just bench` | リリースビルドに対して [bench/run.zsh](../bench/run.zsh) で応答性を計測 |
| `just try` | デバッグビルドを使い、隔離された `ZDOTDIR` と [examples/config.yaml](../examples/config.yaml) で対話 Zsh を起動 |
| `just nix-check` | `nix flake check` でサンドボックス内のビルド・テスト・静的チェックを実行 |
| `just nix-build` | リリース用 Nix パッケージをビルド |

`just try path/to/config.yaml` のように引数を渡すと、任意の設定ファイルで隔離 Zsh を起動できる。

## Nix パッケージと配布構成

外部 Flake 入力は `nixpkgs` のみに限定し、他リポジトリから参照される際の依存グラフを最小に保つ。

- **パッケージ定義（[nix/package.nix](../nix/package.nix)）**: `rustPlatform.buildRustPackage` で `Cargo.lock` からビルドしたバイナリをそのまま配布する。ソースは `lib.fileset` で明示指定しているため、ビルドやテストに必要なディレクトリを新設した場合は `fileset` にも追加する。
- **Home Manager モジュール（[nix/hm-module.nix](../nix/hm-module.nix)）**: `programs.zrs-zsh` でパッケージ導入、`$XDG_CONFIG_HOME/zrs-zsh/config.yaml` の生成、`programs.zsh.initContent` への `eval "$(zrs-zsh init)"` 追加を提供する。
- **Git 追跡の前提**: Nix Flake は Git リポジトリ内の未追跡ファイルを参照しない。新規ファイルを `nix flake check` や `nix build` に反映する際は `git add -N` などで追跡対象にする。
