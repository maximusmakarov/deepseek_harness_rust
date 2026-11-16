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

## Bottles и GHCR

Джобы `build` собирают Homebrew-бутылки (`dshr--<ver>.<tag>.bottle.tar.gz`) для
macOS/Linux, а job `bottles-ghcr` публикует **все платформы одной версии одним
OCI-индексом** `ghcr.io/<owner>/<repo>/dshr:<version>`. Индекс адресуется тегом
версии и перезаписывается при каждом push, поэтому бутылки отправляются одним
вызовом `scripts/publish-bottle-ghcr.py`; пофайловый push оставлял в индексе
только последнюю платформу, и `brew install` ломался на остальных. Индекс
содержит по дочернему манифесту на платформу с аннотациями
`sh.brew.bottle.digest`, `sh.brew.tab` и `org.opencontainers.image.ref.name`.
Нужны `packages: write` и `GITHUB_TOKEN`.

Что проверено:

- mock-регистр: `scripts/test-publish-bottle-ghcr.py` в CI — две платформы
  (`x86_64_linux`, `aarch64_linux`) попадают в один индекс, без потерь и дублей;
- реальный push и анонимный pull в GHCR: run `43585903547` от 14.11.2026
  (тестовый тег `0.0.1`, одна платформа `x86_64_linux`);
- 16.11.2026 анонимный `GET /v2/<owner>/<repo>/dshr/manifests/0.0.1` и
  `tags/list` повторно подтверждены (`{"tags":["0.0.1"]}`), то есть пакет
  публичный и читается без учётных данных;
- сгенерированная формула проходит `brew audit --strict` без замечаний
  (Homebrew 7.0.13). Brew распознаёт `root_url "https://ghcr.io/v2/<owner>/<repo>"`
  как GitHub Packages (`DownloadStrategyDetector` → `CurlGitHubPackagesDownloadStrategy`)
  и запрашивает `…/<formula>/manifests/<version>` — ровно тот путь, куда пишет
  публикатор. Поэтому формула не содержит строки `version`: brew сканирует её
  из URL, а избыточная строка — замечание аудита. Блок `bottle do` сортируется
  (ARM macOS → Intel macOS → ARM Linux → Intel Linux) и выравнивается по
  самому длинному тегу, как требует тот же аудит.

Релизный прогон на теге проверен на `v0.1.0` (16.11.2026):

- `Release` (run `43832256936`) прошёл целиком: релиз содержит 17 ассетов,
  включая `dshr_0.1.0_amd64.deb` (его терял `desktop-linux` до фикса раунда 115),
  `dshr.rb`, `dshr-ghcr.rb` и `SHA256SUMS`;
- индекс `ghcr.io/maximusmakarov/deepseek_harness_rust/dshr:0.1.0` содержит все
  четыре платформы (`arm64_sequoia`, `sonoma`, `aarch64_linux`, `x86_64_linux`) —
  агрегирующий push подтверждён на реальном релизе;
- `brew fetch maximusmakarov/dshr/dshr` скачал манифест индекса и бутылку
  `arm64_sequoia` из GHCR анонимно; sha256 совпал с формулой и с ассетом релиза
  (`1f086b6a…`);
- `brew install maximusmakarov/dshr/dshr` на Apple Silicon собрал из бутылки
  (`poured_from_bottle: true`, receipt указывает на tap `maximusmakarov/dshr`),
  `brew test dshr` и `dshr --version` проходят — установка из GHCR проверена
  end-to-end, не только fetch'ем.

### Tap

Tap существует: публичный `maximusmakarov/homebrew-dshr`, формула `0.1.0`
добавлена через
`scripts/sync-homebrew-tap.sh ../homebrew-dshr --formula dist/dshr-ghcr.rb --push`
(скрипт проверяет `root_url`, bottle-`sha256` и `ruby -c`, коммитит
`dshr <version>`).

Homebrew 7 отказывается загружать формулу из стороннего tap без явного доверия,
поэтому установка начинается с `brew trust`:

```sh
brew trust maximusmakarov/dshr
brew install maximusmakarov/dshr/dshr
brew test dshr
```

Без `brew trust` brew отвечает `Refusing to load formula … from untrusted tap`.
Пакет GHCR должен оставаться публичным: анонимный pull — единственный способ,
которым brew забирает блоб.

## Dry-run релиза без тега

`release.yml` запускается и по `workflow_dispatch` с обязательным входом
`version`: собираются архивы, бутылки и desktop-артефакты (macOS `.app`/DMG,
Linux tar+deb, Windows zip), но `bottles-ghcr` и `publish` пропускаются — пакет
и GitHub Release не создаются. Это единственный способ прогнать desktop-джобы,
не выпуская релиз. Версия нормализуется `scripts/release-version.sh`
(`v0.1.0` и `0.1.0` → `0.1.0`, мусор отвергается).

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
- `dshr.rb` (GitHub Releases) и `dshr-ghcr.rb` — бинарные формулы с блоками
  `bottle do`, а не `brew bottle`-выгрузки: бутылки собирает
  `scripts/package-bottle.sh`, поэтому они не сверяются с `brew bottle --merge`.
- Релизный прогон на теге ещё не выполнялся: desktop-джобы и `publish`
  проверяются только через `workflow_dispatch`-dry-run и локальные dry-run'ы.
- Ключ подписи один; ротации и отзыва ключей нет.
