# zrs

Zsh のコマンドライン上で、YAML で定義した略語展開・スニペット挿入・カスタムファジー補完を行うツール。

Space や Enter での略語展開（コンテキスト条件やコマンド評価を含む）、プレースホルダー付きスニペットの対話挿入、外部コマンドを候補源とするファジー補完に対応する。

## Requirements

- Zsh
- fzf

## Configuration

設定ファイルは `$XDG_CONFIG_HOME/zrs/config.yaml`（`$XDG_CONFIG_HOME` 未設定時は `~/.config/zrs/config.yaml`）に置く。

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
- [development.md](docs/development.md)
