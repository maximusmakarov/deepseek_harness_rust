# Релизы

Тег `v<version>` запускает [release.yml](../.github/workflows/release.yml). Пайплайн
собирает пять архивов, аттестует их, объединяет контрольные суммы, подписывает их
(если задан ключ), рендерит бинарную Homebrew-формулу и создаёт GitHub Release.

| Артефакт | Назначение |
|---|---|
| `dshr-<version>-<target>.tar.gz` | бинарь + README + лицензии |
| `SHA256SUMS` | суммы всех архивов |
| `SHA256SUMS.sig` | Ed25519-подпись `SHA256SUMS` (при наличии ключа) |
| `SHA256SUMS.pub` | публичный ключ подписи |
| `dshr.rb` | бинарная Homebrew-формула с закреплёнными `sha256` |
| `dshr-<version>-macos.zip` | подписанный ad-hoc `.app` (Tauri + `dshr`) |

Таргеты: `x86_64`/`aarch64-apple-darwin`, `x86_64`/`aarch64-unknown-linux-gnu`,
`x86_64-pc-windows-msvc`. Linux `aarch64` собирается через `cross`. macOS-джоб
дополнительно собирает `dshr.app` (`scripts/package-desktop.sh`) и кладёт в zip.

## Проверка

```sh
shasum -a 256 -c SHA256SUMS                 # или sha256sum -c
dshr release verify SHA256SUMS \
  --signature SHA256SUMS.sig \
  --key "$(cat SHA256SUMS.pub)"
gh attestation verify dshr-<version>-<target>.tar.gz \
  --repo maximusmakarov/deepseek_harness_rust
```

Подпись опциональна: если `DSHR_RELEASE_KEY` не задан, публикуются только суммы и
provenance. Отсутствие `.sig` видно по релизу и не маскируется.

## Ключ подписи

```sh
dshr release keygen --out release.key       # печатает публичный ключ
gh secret set DSHR_RELEASE_KEY < release.key
```

Приватный ключ — 32-байтная hex-строка; файл создаётся с правами `0600`. Публичный
ключ первого релиза стоит продублировать в этом документе, чтобы его можно было
проверить до скачивания артефактов.

## Локальная сборка архива

```sh
cargo build --release -p dshr-cli
scripts/package-release.sh 0.1.0 aarch64-apple-darwin
scripts/render-formula.sh 0.1.0 SHA256SUMS dshr.rb   # нужны все 4 таргета
```

`scripts/install.sh` умеет ставить из локального каталога:
`DSHR_RELEASE_BASE_URL=file://$PWD/dist DSHR_VERSION=0.1.0 DSHR_TARGET=... scripts/install.sh`.

## Границы

- Нет воспроизводимых сборок: архивы зависят от toolchain и окружения раннера.
- Нет ghcr-бутылок Homebrew: `dshr.rb` — бинарная формула, а не `brew bottle`.
- Ключ подписи один; ротации и отзыва ключей нет.
