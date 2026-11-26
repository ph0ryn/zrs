# Benchmark

`zrs`、`zsh-abbr`、`zeno.zsh` の 3 ツールを同じ条件で比較した実測結果。

## Zsh 起動時間

`zsh -di -c exit`（100 件設定・キャッシュ済み）の所要時間。

| Tool | p50 | p95 | p99 | min | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| bare zsh | 6.36ms | 7.20ms | 7.26ms | 5.67ms | 7.26ms |
| zrs | **16.18ms** | **16.94ms** | **17.40ms** | **14.84ms** | **17.40ms** |
| zeno.zsh (no deno cache) | 31.97ms | 61.55ms | 71.86ms | 29.94ms | 71.86ms |
| zsh-abbr (cached) | 48.18ms | 52.19ms | 56.28ms | 44.62ms | 56.28ms |
| zeno.zsh (default) | 67.84ms | 122.40ms | 168.49ms | 63.93ms | 168.49ms |

## 略語展開・不一致レイテンシ（100 件設定）

Space 押下時の展開処理を、略語に一致する場合（`gs`）と一致しない場合（`echo hello`）で計測した結果。

| Tool | hit p50 | hit p95 | hit p99 | miss p50 | miss p95 | miss p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| zrs | 3.05ms | 4.14ms | 6.14ms | 2.43ms | **2.84ms** | **3.37ms** |
| zsh-abbr | 3.11ms | 4.65ms | 5.97ms | 6.43ms | 8.11ms | 9.12ms |
| zeno.zsh | **2.53ms** | **3.36ms** | **3.99ms** | **2.42ms** | 2.88ms | 3.44ms |

## アイドル時メモリ（RSS）

1 回展開を実行した直後の常駐メモリ使用量（100 件設定）。

| Tool | Shell RSS | Daemon RSS | Total RSS |
| --- | ---: | ---: | ---: |
| bare zsh | 2.8 MB | 0.0 MB | 2.8 MB |
| zrs | **3.2 MB** | **0.0 MB** | **3.2 MB** |
| zsh-abbr | 4.6 MB | **0.0 MB** | 4.6 MB |
| zeno.zsh | 4.4 MB | 71.8 MB | 76.2 MB |

## コールドスタート（初回展開）

新しいシェルを開いて最初に 1 回だけ略語を展開したときの応答時間（100 件設定）。

| Tool | p50 | p95 | max |
| --- | ---: | ---: | ---: |
| zrs | **3.10ms** | **3.43ms** | **3.43ms** |
| zsh-abbr | 3.24ms | 3.53ms | 3.53ms |
| zeno.zsh | 121.42ms | 123.04ms | 123.04ms |

## 設定変更の反映

シェルを開いたまま設定ファイルに新しい略語を追記した直後の挙動。

| Tool | 設定ファイル編集直後の反映 |
| --- | --- |
| zrs | **即時反映（リロード不要）** |
| zsh-abbr | 未反映（`abbr-load` またはシェル再起動が必要） |
| zeno.zsh | 未反映（`zeno-restart-server` 等が必要） |

## スケーラビリティ（100 / 1,000 / 10,000 件）

略語件数を増やしたときの応答時間の変化。

| 件数 | Tool | hit p50 | hit p95 | miss p50 | miss p95 |
| ---: | --- | ---: | ---: | ---: | ---: |
| 100 | zrs | 2.77ms | 3.55ms | 2.64ms | 3.31ms |
| 100 | zsh-abbr | 2.93ms | 3.62ms | 6.65ms | 7.74ms |
| 100 | zeno.zsh | **2.06ms** | **2.40ms** | **2.41ms** | **3.17ms** |
| 1,000 | zrs | 4.32ms | 5.56ms | 4.27ms | 5.11ms |
| 1,000 | zsh-abbr | 3.77ms | 4.14ms | 7.29ms | 7.77ms |
| 1,000 | zeno.zsh | **2.77ms** | **3.41ms** | **2.44ms** | **2.93ms** |
| 10,000 | zrs | 21.28ms | 22.82ms | 21.41ms | 23.79ms |
| 10,000 | zsh-abbr | 13.09ms | 14.19ms | 19.78ms | 20.63ms |
| 10,000 | zeno.zsh | **2.34ms** | **5.50ms** | **2.68ms** | **5.61ms** |

## 計測環境と再現方法

- OS / CPU: Darwin 27.0.0 arm64
- Zsh: `zsh 5.9.2 (aarch64-apple-darwin25.6.0)`
- `zrs`: `zrs-zsh 0.1.0`（release build）
- `zsh-abbr`: `889f477`
- `zeno.zsh`: `f6ed8fc`（`deno 2.9.7`）
- 設定条件: 3 ツールに等価な略語セット（先頭に `gs -> git status --short --branch`、以降 5 件に 1 件をグローバル展開、残りを通常展開）を自動生成
- 試行回数: 起動 20 回、100 件 100 回（warmup 10）、1,000 件 30 回（warmup 5）、10,000 件 10 回（warmup 2）、コールドスタート 5 回

手元の環境で再計測する場合は [bench/compare.zsh](../bench/compare.zsh) を実行する。

```sh
cargo build --release
./bench/compare.zsh
```
