# zrs

常駐プロセス不要で高速に動作する、Rust 製の Zsh 向け略語展開・スニペット挿入・履歴検索・カスタムファジー補完ツール。

## Requirements

- [fzf](https://github.com/junegunn/fzf): スニペット選択・履歴検索・カスタム補完を使う場合のみ

## Installation

インストール後、`.zshrc` に以下を追加する（Home Manager の Zsh 統合を使う場合は不要）。
`compinit` や `fzf-tab` の後に配置する。`zsh-vi-mode` と併用する場合は `ZVM_INIT_MODE=sourcing` を設定して読み込んだ後に実行する。

```zsh
eval "$(zrs-zsh init)"
```

既定のキーバインドを設定せずウィジェット定義のみ行う場合は、`--no-keybinds` を渡す。

```zsh
eval "$(zrs-zsh init --no-keybinds)"
```

### Cargo

Rust 1.89 以降の環境で crates.io からインストールする。

```sh
cargo install zrs-zsh
```

### GitHub Releases

[Releases](https://github.com/ph0ryn/zrs/releases) から環境向けのアーカイブをダウンロードして展開し、バイナリを `PATH` の通ったディレクトリへ配置する。

### Nix

Flake から直接インストールする。

```sh
nix profile install github:ph0ryn/zrs
```

Home Manager で導入する場合は、`flake.nix` の `inputs` に `zrs-zsh.url = "github:ph0ryn/zrs";` を追加し、モジュール側で `homeManagerModules.default` を読み込む。
既定で Zsh 統合（`enableZshIntegration`）が有効になり、`settings` で設定ファイルも生成できる。

```nix
{ inputs, ... }:
{
  imports = [ inputs.zrs-zsh.homeManagerModules.default ];

  programs.zrs-zsh = {
    enable = true;
    enableZshIntegration = true;
    settings = {
      abbreviations = [
        {
          keyword = "gs";
          expand = "git status --short --branch";
        }
      ];
    };
  };
}
```

## Features

- **非常駐・高速** — 常駐デーモンを持たない Rust 製単一バイナリ。キー入力ごとのワンショット実行で、設定 100 件規模でも Zsh 経由の略語展開が約 2.5〜3 ms で完了する（競合比較は [benchmark.md](docs/benchmark.md) を参照）。
- **略語展開** — `Space` や `Enter` で略語を展開する。通常展開、グローバル展開、コンテキスト条件展開、コマンド評価展開、プレースホルダーに対応する。
- **スニペット挿入** — `^X s` で登録済みスニペットを `fzf` で検索し、カーソル位置へ挿入する。
- **履歴検索** — `^R` で Zsh のコマンド履歴を `fzf` で絞り込み、選択したコマンドを入力行へ復元する。
- **カスタムファジー補完** — `Tab` で入力行のパターンに応じた外部コマンドの出力を `fzf` で補完する。プレビュー表示と複数選択に対応する。
- **設定検証** — `zrs-zsh validate` で YAML 設定の構文や定義を事前に検証できる。

## Configuration

設定ファイルは `$XDG_CONFIG_HOME/zrs-zsh/config.yaml`（`$XDG_CONFIG_HOME` 未設定時は `~/.config/zrs-zsh/config.yaml`）に置く。

`abbreviations`、`snippets`、`completions` はいずれも省略できる。複数ルールに一致した場合は記述順の先頭を使う。

```yaml
abbreviations:
  - keyword: gs
    expand: git status --short --branch

  - keyword: B
    expand: git symbolic-ref --short HEAD
    context:
      lbuffer: '^git\s+checkout\s+'
    evaluate: true

snippets:
  - name: docker run
    body: docker run --rm -it '{{image}}'

completions:
  - name: git switch
    pattern: '^git switch\s'
    source: git branch --format='%(refname:short)'
    preview: git log --oneline -20 {}
```

各フィールドの必須・任意や条件評価の詳細は [設定仕様](docs/spec.md) を参照。

## Documentation

- [spec.md](docs/spec.md)
- [architecture.md](docs/architecture.md)
- [verification.md](docs/verification.md)
- [benchmark.md](docs/benchmark.md)
- [development.md](docs/development.md)

## License

[MIT](LICENSE)
