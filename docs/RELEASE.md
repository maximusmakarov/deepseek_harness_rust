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
| `dshr-<version>-macos.zip` | `.app` (ad-hoc или Developer ID + notarization) |
| `dshr-<version>.dmg` | установочный DMG (`.app` + Applications) |
| `dshr--<version>.<tag>.bottle.tar.gz` | Homebrew bottle для macOS/Linux (GitHub Releases) |
| `dshr-<version>-linux-desktop.tar.gz` | `dshr` + `dshr-desktop` (WebKitGTK build) |
| `dshr_<version>_amd64.deb` | Debian-пакет CLI+desktop (WebKitGTK depends) |
| `dshr-<version>-windows-desktop.zip` | `dshr.exe` + `dshr-desktop.exe` |

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

## Dry-run (macOS)

Локально проверены: детерминированный tar.gz + sha256, bottle-layout,
`bottle do` (ruby -c), установка через install.sh (file://) и `dshr --version`.

## Bottles

Release-джобы собирают Homebrew-бутылки (`dshr--<ver>.<tag>.bottle.tar.gz`)
для macOS/Linux; job `bottles-ghcr` публикует их в GHCR как OCI-индекс
(`ghcr.io/<owner>/<repo>/dshr:<ver>`), а `dist/dshr-ghcr.rb` содержит
`root_url https://ghcr.io/v2/<owner>/<repo>`. Требуются `packages: write`
и `GITHUB_TOKEN`; dry-run OCI-layout, mock-push и выбор манифеста Homebrew проверены локально; реальный push в GHCR и анонимный pull индекса подтверждены в Actions (run 43585903547). Полный pour с локального OCI невозможен — brew тянет blobs с ghcr.io.

Release-джобы собирают Homebrew-бутылки (`dshr--<ver>.<tag>.bottle.tar.gz`)
для macOS/Linux и добавляют `bottle do` в формулу. `root_url` указывает на
GitHub Releases, не GHCR; pour проверяется при следующем теге.

## Bottle packaging

`package-bottle.sh` создаёт выходной каталог до записи архива; macOS/Linux
сборка бутылки проверена в Actions.

## Desktop CI

Linux/Windows desktop-сборка и упаковка проверены в Actions (tar+deb, zip);
Windows-ресурс использует `crates/desktop/icons/icon.ico`, `window-vibrancy`
подключён только для macOS.

## Notarization (macOS)

Секреты `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`, `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `DSHR_CODESIGN_IDENTITY` включают импорт
сертификата, Developer ID-подпись и `notarytool submit --wait` + staple.
Без них `.app` подписывается ad-hoc.

## Воспроизводимость

Сборка с `SOURCE_DATE_EPOCH=0`, `CARGO_INCREMENTAL=0` и
`RUSTFLAGS="--remap-path-prefix=$PWD=/dshr --remap-path-prefix=$PWD/target=/target -C debuginfo=0"`
даёт побайтово одинаковый бинарь при том же toolchain: проверка
`scripts/check-reproducible.sh` сравнивает два независимых target-каталога.
CI использует эти переменные; `cross`-сборка Linux aarch64 может отличаться
путями внутри контейнера.

## Границы

- Архивы детерминированы (одинаковый бинарь → одинаковый sha256), но сами бинари
  не воспроизводимы побайтово: зависят от toolchain и окружения раннера.
- Нет ghcr-бутылок Homebrew: `dshr.rb` — бинарная формула, а не `brew bottle`.
- Ключ подписи один; ротации и отзыва ключей нет.
